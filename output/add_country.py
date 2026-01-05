#!/usr/bin/env python3
"""
Script to extract country from addresses and add to exhibitor data.
"""

import csv
import json
from pathlib import Path

def extract_country(address: str) -> str:
    """Extract country from address string (typically the last part after comma)."""
    if not address or not address.strip():
        return ""

    # Split by comma and get the last part
    parts = [p.strip() for p in address.split(',')]
    if parts:
        country = parts[-1].strip()
        # Handle edge cases where country might be empty or just whitespace
        if country:
            return country
    return ""


def process_csv(input_path: str, output_path: str):
    """Process CSV file and add country column."""
    with open(input_path, 'r', encoding='utf-8') as infile:
        reader = csv.reader(infile)
        rows = list(reader)

    if not rows:
        return

    # Find the Address column index and Country column index
    header = rows[1]  # Row 0 is empty, row 1 is header
    address_idx = header.index('Address') if 'Address' in header else -1
    country_idx = header.index('Country') if 'Country' in header else -1

    if address_idx == -1:
        print("Error: Address column not found")
        return

    # Process each row
    processed_rows = []
    for i, row in enumerate(rows):
        if i <= 1:  # Keep header rows as-is
            processed_rows.append(row)
            continue

        if len(row) > address_idx:
            address = row[address_idx]
            country = extract_country(address)

            # Update or add country column
            if country_idx != -1 and len(row) > country_idx:
                row[country_idx] = country
            elif country_idx != -1:
                # Extend row if needed
                while len(row) <= country_idx:
                    row.append('')
                row[country_idx] = country

        processed_rows.append(row)

    # Write output
    with open(output_path, 'w', encoding='utf-8', newline='') as outfile:
        writer = csv.writer(outfile)
        writer.writerows(processed_rows)

    print(f"CSV processed: {len(processed_rows) - 2} exhibitors")


def process_json(input_path: str, output_path: str):
    """Process JSON file and add country field."""
    with open(input_path, 'r', encoding='utf-8') as infile:
        data = json.load(infile)

    exhibitors = data.get('exhibitors', [])

    for exhibitor in exhibitors:
        address = exhibitor.get('address', '')
        country = extract_country(address)
        exhibitor['country'] = country

    # Write output
    with open(output_path, 'w', encoding='utf-8') as outfile:
        json.dump(data, outfile, indent=2, ensure_ascii=False)

    print(f"JSON processed: {len(exhibitors)} exhibitors")


def main():
    base_dir = Path(__file__).parent

    # Process CSV
    csv_input = base_dir / 'all_exhibitors.csv'
    csv_output = base_dir / 'all_exhibitors_with_country.csv'
    process_csv(str(csv_input), str(csv_output))

    # Process JSON
    json_input = base_dir / 'all_exhibitors.json'
    json_output = base_dir / 'all_exhibitors_with_country.json'
    process_json(str(json_input), str(json_output))

    print("\nDone! Created:")
    print(f"  - {csv_output}")
    print(f"  - {json_output}")


if __name__ == '__main__':
    main()
