use anyhow::Result;
use chrono::Utc;
use csv::Writer;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

const API_URL: &str = "https://exhibitors.ces.tech/8_0/ajax/remote-proxy.cfm";
const PAGE_SIZE: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Exhibitor {
    name: String,
    exhid: String,
    detail_url: String,
    booth_venue: String,
    booth_number: String,
    booth_full: String,
    description: String,
    website: String,
    address: String,
    product_categories: String,
    hall_ids: String,
    seek_funding: String,
    funding_amount: String,
    revenue: String,
    investment_stage: String,
    scraped_at: String,
}

impl Default for Exhibitor {
    fn default() -> Self {
        Exhibitor {
            name: String::new(),
            exhid: String::new(),
            detail_url: String::new(),
            booth_venue: String::new(),
            booth_number: String::new(),
            booth_full: String::new(),
            description: String::new(),
            website: String::new(),
            address: String::new(),
            product_categories: String::new(),
            hall_ids: String::new(),
            seek_funding: String::new(),
            funding_amount: String::new(),
            revenue: String::new(),
            investment_stage: String::new(),
            scraped_at: String::new(),
        }
    }
}

fn clean_booth(booth: &str) -> String {
    booth.replace("randomstring", "").trim().to_string()
}

fn parse_exhibitor(hit: &Value) -> Exhibitor {
    let fields = &hit["fields"];

    let mut exhibitor = Exhibitor::default();
    exhibitor.scraped_at = Utc::now().to_rfc3339();

    // Name
    exhibitor.name = fields["exhname_t"].as_str().unwrap_or("").to_string();

    // Exhibitor ID
    exhibitor.exhid = fields["exhid_l"].as_str().unwrap_or("").to_string();

    // Detail URL
    exhibitor.detail_url = format!(
        "https://exhibitors.ces.tech/8_0/exhibitor/exhibitor-details.cfm?exhid={}",
        exhibitor.exhid
    );

    // Description - this is the key field!
    exhibitor.description = fields["exhdesc_t"].as_str().unwrap_or("").to_string();

    // Booths - just store booth numbers for now, venue will come from detail page
    if let Some(booths) = fields["boothsdisplay_la"].as_array() {
        let booth_strs: Vec<String> = booths
            .iter()
            .filter_map(|b| b.as_str())
            .map(|b| clean_booth(b))
            .filter(|b| !b.is_empty())
            .collect();
        exhibitor.booth_number = booth_strs.join("; ");
    }

    // Hall IDs
    if let Some(halls) = fields["hallid_la"].as_array() {
        let hall_strs: Vec<String> = halls
            .iter()
            .filter_map(|h| h.as_str())
            .map(|h| h.to_string())
            .collect();
        exhibitor.hall_ids = hall_strs.join("; ");
    }

    // Funding info
    exhibitor.seek_funding = fields["seekfunding_t"].as_str().unwrap_or("").to_string();
    exhibitor.funding_amount = fields["fundingamount_t"]
        .as_str()
        .unwrap_or("")
        .replace(";", ",")
        .to_string();
    exhibitor.revenue = fields["revenue_t"]
        .as_str()
        .unwrap_or("")
        .replace(";", ",")
        .to_string();
    exhibitor.investment_stage = fields["investmentstage_l"].as_str().unwrap_or("").to_string();

    exhibitor
}

async fn fetch_exhibitors_page(client: &Client, start: usize, size: usize) -> Result<(Vec<Exhibitor>, usize)> {
    let url = format!(
        "{}?action=search&searchtype=exhibitorgallery&searchsize={}&start={}",
        API_URL, size, start
    );

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .header("Referer", "https://exhibitors.ces.tech/8_0/explore/exhibitor-gallery.cfm")
        .header("X-Requested-With", "XMLHttpRequest")
        .send()
        .await?
        .json::<Value>()
        .await?;

    let success = response["SUCCESS"].as_bool().unwrap_or(false);
    if !success {
        return Err(anyhow::anyhow!("API returned unsuccessful response"));
    }

    let total = response["DATA"]["totalhits"].as_f64().unwrap_or(0.0) as usize;

    let mut exhibitors = Vec::new();

    // Parse exhibitor hits
    if let Some(hits) = response["DATA"]["results"]["exhibitor"]["hit"].as_array() {
        for hit in hits {
            exhibitors.push(parse_exhibitor(hit));
        }
    }

    // Also parse featured hits
    if let Some(hits) = response["DATA"]["results"]["featured"]["hit"].as_array() {
        for hit in hits {
            let ex = parse_exhibitor(hit);
            // Avoid duplicates
            if !exhibitors.iter().any(|e| e.exhid == ex.exhid) {
                exhibitors.push(ex);
            }
        }
    }

    Ok((exhibitors, total))
}

async fn fetch_exhibitor_details(client: &Client, exhibitor: &mut Exhibitor) -> Result<()> {
    let url = &exhibitor.detail_url;

    let response = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
        .send()
        .await?
        .text()
        .await?;

    // Extract website from Vue data: websiteValue: "https://..."
    let website_re = Regex::new(r#"websiteValue:\s*"([^"]+)""#).unwrap();
    if let Some(caps) = website_re.captures(&response) {
        let website = caps.get(1).map_or("", |m| m.as_str())
            .replace("\\/", "/");
        exhibitor.website = website;
    }

    // Extract address from Vue data: addressValues: {"ZIP":"...","CITY":"...","ADDRESS1":"...",...}
    let addr_re = Regex::new(r#"addressValues:\s*(\{[^}]+\})"#).unwrap();
    if let Some(caps) = addr_re.captures(&response) {
        if let Some(json_str) = caps.get(1) {
            if let Ok(addr_json) = serde_json::from_str::<Value>(json_str.as_str()) {
                let mut parts = Vec::new();
                if let Some(addr1) = addr_json["ADDRESS1"].as_str() {
                    if !addr1.is_empty() { parts.push(addr1.to_string()); }
                }
                if let Some(addr2) = addr_json["ADDRESS2"].as_str() {
                    if !addr2.is_empty() { parts.push(addr2.to_string()); }
                }
                if let Some(city) = addr_json["CITY"].as_str() {
                    if !city.is_empty() { parts.push(city.to_string()); }
                }
                if let Some(state) = addr_json["STATE"].as_str() {
                    if !state.is_empty() && state != "-" { parts.push(state.to_string()); }
                }
                if let Some(zip) = addr_json["ZIP"].as_str() {
                    if !zip.is_empty() { parts.push(zip.to_string()); }
                }
                if let Some(country) = addr_json["COUNTRY"].as_str() {
                    if !country.is_empty() { parts.push(country.to_string()); }
                }
                exhibitor.address = parts.join(", ");
            }
        }
    }

    // Extract booth venue from floorplan link: LVCC, Central Hall &mdash; 17214
    let booth_re = Regex::new(r#"(LVCC[^&<]+|Venetian[^&<]+|Westgate[^&<]+|Aria[^&<]+|Wynn[^&<]+|Fontainebleau[^&<]+|Resorts World[^&<]+)\s*(?:&mdash;|—|-)\s*(\d+[a-zA-Z]?)"#).unwrap();
    if let Some(caps) = booth_re.captures(&response) {
        let venue = caps.get(1).map_or("", |m| m.as_str()).trim().to_string();
        let booth_num = caps.get(2).map_or("", |m| m.as_str()).to_string();
        exhibitor.booth_venue = venue.clone();
        exhibitor.booth_full = format!("{} - {}", venue, booth_num);
    } else {
        // Try alternate pattern for hospitality suites
        let alt_re = Regex::new(r#">([\w\s]+Hospitality Suites?[^<]*)<"#).unwrap();
        if let Some(caps) = alt_re.captures(&response) {
            let venue = caps.get(1).map_or("", |m| m.as_str()).trim().to_string();
            exhibitor.booth_venue = venue.clone();
            exhibitor.booth_full = venue;
        }
    }

    // Extract categories from detail page
    let cat_re = Regex::new(r#"searchtype/category/search/\d+/show/all">([^<]+)<"#).unwrap();
    let categories: Vec<String> = cat_re.captures_iter(&response)
        .filter_map(|caps| {
            let text = caps.get(1).map_or("", |m| m.as_str()).trim().to_string();
            if !text.is_empty() && text != "Product Categories" && text.len() > 2 {
                Some(text)
            } else {
                None
            }
        })
        .collect();
    if !categories.is_empty() {
        exhibitor.product_categories = categories.join("; ");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== CES 2026 Exhibitor Scraper (Rust - Fast API Mode) ===\n");

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    // Step 1: Fetch first page to get total count
    println!("Fetching exhibitor count...");
    let (_, total) = fetch_exhibitors_page(&client, 0, 1).await?;
    println!("Total exhibitors: {}\n", total);

    // Step 2: Fetch all exhibitors via API pagination
    println!("=== Phase 1: Fetching exhibitor data via API ===");

    let mut all_exhibitors: Vec<Exhibitor> = Vec::new();
    let mut start = 0;

    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    while start < total {
        match fetch_exhibitors_page(&client, start, PAGE_SIZE).await {
            Ok((exhibitors, _)) => {
                let count = exhibitors.len();
                all_exhibitors.extend(exhibitors);
                pb.inc(count as u64);
                start += PAGE_SIZE;
            }
            Err(e) => {
                eprintln!("\nError fetching page at start={}: {}", start, e);
                start += PAGE_SIZE;
            }
        }

        // Small delay to be nice to the server
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    pb.finish_with_message("API fetch complete");

    // Deduplicate by exhid
    let mut seen = std::collections::HashSet::new();
    all_exhibitors.retain(|e| seen.insert(e.exhid.clone()));

    println!("\nUnique exhibitors fetched: {}", all_exhibitors.len());

    // Step 3: Fetch additional details from detail pages
    println!("\n=== Phase 2: Fetching details from {} detail pages ===", all_exhibitors.len());

    let pb2 = ProgressBar::new(all_exhibitors.len() as u64);
    pb2.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    // Use parallel requests for detail pages
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(30));
    let client = std::sync::Arc::new(client);

    let mut handles = Vec::new();

    for mut exhibitor in all_exhibitors.into_iter() {
        let client = client.clone();
        let semaphore = semaphore.clone();
        let pb = pb2.clone();

        let handle = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            let _ = fetch_exhibitor_details(&client, &mut exhibitor).await;
            pb.inc(1);
            exhibitor
        });

        handles.push(handle);
    }

    let mut final_exhibitors = Vec::new();
    for handle in handles {
        if let Ok(ex) = handle.await {
            final_exhibitors.push(ex);
        }
    }

    pb2.finish_with_message("Detail fetch complete");

    // Stats
    let total = final_exhibitors.len();
    let has_desc = final_exhibitors.iter().filter(|e| e.description.len() > 20).count();
    let has_booth = final_exhibitors.iter().filter(|e| !e.booth_full.is_empty()).count();
    let has_venue = final_exhibitors.iter().filter(|e| !e.booth_venue.is_empty()).count();
    let has_website = final_exhibitors.iter().filter(|e| !e.website.is_empty()).count();
    let has_address = final_exhibitors.iter().filter(|e| !e.address.is_empty()).count();
    let has_cats = final_exhibitors.iter().filter(|e| !e.product_categories.is_empty()).count();

    println!("\n=== Results ===");
    println!("Total exhibitors: {}", total);
    println!("With description: {} ({}%)", has_desc, if total > 0 { has_desc * 100 / total } else { 0 });
    println!("With booth_venue: {} ({}%)", has_venue, if total > 0 { has_venue * 100 / total } else { 0 });
    println!("With booth_full: {} ({}%)", has_booth, if total > 0 { has_booth * 100 / total } else { 0 });
    println!("With website: {} ({}%)", has_website, if total > 0 { has_website * 100 / total } else { 0 });
    println!("With address: {} ({}%)", has_address, if total > 0 { has_address * 100 / total } else { 0 });
    println!("With categories: {} ({}%)", has_cats, if total > 0 { has_cats * 100 / total } else { 0 });

    // Save output
    std::fs::create_dir_all("output")?;

    // Save JSON
    let output = serde_json::json!({
        "total_count": total,
        "with_description": has_desc,
        "with_categories": has_cats,
        "with_website": has_website,
        "with_address": has_address,
        "scraped_at": Utc::now().to_rfc3339(),
        "exhibitors": final_exhibitors
    });
    std::fs::write(
        "output/all_exhibitors.json",
        serde_json::to_string_pretty(&output)?,
    )?;
    println!("\nSaved to output/all_exhibitors.json");

    // Save CSV
    let mut wtr = Writer::from_path("output/all_exhibitors.csv")?;
    for ex in &final_exhibitors {
        wtr.serialize(ex)?;
    }
    wtr.flush()?;
    println!("Saved to output/all_exhibitors.csv");

    Ok(())
}
