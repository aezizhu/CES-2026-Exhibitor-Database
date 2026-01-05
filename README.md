# CES 2026 Exhibitor Scraper

A high-performance web scraper written in Rust to collect exhibitor data from CES 2026 (Consumer Electronics Show). This tool fetches comprehensive exhibitor information including company details, booth locations, product categories, and funding information.

## Features

- **Fast API-based scraping**: Uses the official CES exhibitor API for bulk data retrieval
- **Concurrent detail fetching**: Parallel requests with configurable concurrency (30 simultaneous requests)
- **Comprehensive data extraction**: Collects 15+ fields per exhibitor
- **Dual output formats**: Exports to both JSON and CSV
- **Progress tracking**: Real-time progress bars with ETA
- **Rate limiting**: Built-in delays to respect server resources
- **Deduplication**: Automatic removal of duplicate exhibitors

## Data Fields Collected

| Field | Description |
|-------|-------------|
| `name` | Company/exhibitor name |
| `exhid` | Unique exhibitor ID |
| `detail_url` | Link to exhibitor detail page |
| `booth_venue` | Venue location (LVCC, Venetian, etc.) |
| `booth_number` | Booth number |
| `booth_full` | Complete booth location string |
| `description` | Company description |
| `website` | Company website URL |
| `address` | Full company address |
| `product_categories` | Product categories (semicolon-separated) |
| `hall_ids` | Hall location IDs |
| `seek_funding` | Whether seeking investment |
| `funding_amount` | Funding amount sought |
| `revenue` | Company revenue range |
| `investment_stage` | Current investment stage |
| `scraped_at` | Timestamp of data collection |

## Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Internet connection

## Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/ces_2026_rust_scraper.git
cd ces_2026_rust_scraper
```

2. Build the project:
```bash
cargo build --release
```

## Usage

Run the scraper:
```bash
cargo run --release
```

The scraper operates in two phases:
1. **Phase 1**: Fetches exhibitor list via API pagination (100 exhibitors per request)
2. **Phase 2**: Enriches data by scraping individual exhibitor detail pages

Output files are saved to the `output/` directory:
- `all_exhibitors.json` - Full JSON export with metadata
- `all_exhibitors.csv` - CSV export for spreadsheet use

## Example Output

```
=== CES 2026 Exhibitor Scraper (Rust - Fast API Mode) ===

Fetching exhibitor count...
Total exhibitors: 4500

=== Phase 1: Fetching exhibitor data via API ===
[00:00:45] [########################################] 4500/4500 (0s)
API fetch complete

Unique exhibitors fetched: 4500

=== Phase 2: Fetching details from 4500 detail pages ===
[00:02:30] [########################################] 4500/4500 (0s)
Detail fetch complete

=== Results ===
Total exhibitors: 4500
With description: 4200 (93%)
With booth_venue: 4100 (91%)
With booth_full: 4100 (91%)
With website: 3800 (84%)
With address: 4000 (88%)
With categories: 3500 (77%)

Saved to output/all_exhibitors.json
Saved to output/all_exhibitors.csv
```

## Project Structure

```
ces_2026_rust_scraper/
├── Cargo.toml          # Project dependencies
├── Cargo.lock          # Locked dependency versions
├── src/
│   └── main.rs         # Main application code
├── output/             # Generated output files
│   ├── all_exhibitors.json
│   └── all_exhibitors.csv
└── README.md
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime |
| `reqwest` | HTTP client |
| `scraper` | HTML parsing |
| `serde` / `serde_json` | JSON serialization |
| `csv` | CSV export |
| `futures` | Async utilities |
| `indicatif` | Progress bars |
| `chrono` | Timestamps |
| `anyhow` | Error handling |
| `regex` | Pattern matching |

## Configuration

Key constants in `src/main.rs`:

```rust
const API_URL: &str = "https://exhibitors.ces.tech/8_0/ajax/remote-proxy.cfm";
const PAGE_SIZE: usize = 100;  // Exhibitors per API request
```

Concurrency is controlled by a semaphore (default: 30 concurrent requests):
```rust
let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(30));
```

## Performance

- **API Phase**: ~45 seconds for 4500 exhibitors
- **Detail Phase**: ~2-3 minutes with 30 concurrent requests
- **Total runtime**: ~3-4 minutes for complete scrape

## Legal Disclaimer

This tool is provided for educational and research purposes only. Users are responsible for ensuring compliance with:
- CES website terms of service
- Applicable data protection regulations
- Rate limiting and respectful scraping practices

## License

MIT License

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## Acknowledgments

- CES (Consumer Electronics Show) for providing the exhibitor platform
- The Rust async ecosystem for excellent tooling
