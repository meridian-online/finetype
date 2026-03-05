# Locale Support Guide

## What is Locale Support?

FineType recognizes **region-specific data formats**. A phone number valid in the US (+1 202 555 0100) looks different from a UK number (+44 20 7946 0958). A postal code in Canada (M5V 3A8) is invalid in the US (90210).

**Locale support** means FineType:
1. **Detects** which region a value is from
2. **Validates** using region-specific patterns
3. **Returns locale information** so you know what region the data represents

This guide explains what you can do with FineType's locale capabilities and how to use them.

## Current Locale Coverage (v0.5.3)

As of March 2026, FineType supports 65+ locales across validation and generation:

### Fully Supported Types (Validation + Generation)

| Type | Locales | Example |
|------|---------|---------|
| **postal_code** | 65 locales | US (90210), GB (SW1A 1AA), CA (M5V 3A8), DE (10115), FR (75001), JP (100-0001), BR (01310-100), IN (110001), [+57 more] |
| **phone_number** | 46 locales | US (+1 202 555 0100), DE (+49 30 12345678), FR (+33 1 42 68 53 00), KR (+82 2 1234 5678), ZA (+27 11 960 2000), [+41 more] |
| **month_name** | 27 locales | January (EN), janvier (FR), Januar (DE), enero (ES), gennaio (IT), janeiro (PT), январь (RU), январь (UK), Январь (BG), [+18 more] |
| **day_of_week** | 27 locales | Monday (EN), lundi (FR), Montag (DE), lunes (ES), lunedì (IT), segunda-feira (PT), понедельник (RU), пн (UK), понеделник (BG), [+18 more] |
| **calling_code** | 17 locales | +1 (US/CA), +44 (GB), +49 (DE), +33 (FR), +39 (IT), +31 (NL), +34 (ES), +358 (FI), +46 (SE), +47 (NO), [+7 more] |

### Partial Support (Validation Only)

These types have region-specific validation but no generation:

- **address** (full_address, street_name) — 8 locales — validation patterns for major regions
- **name types** — currently universal (not locale-specific), future expansion planned

## How Locale Detection Works

When you classify a column, FineType returns both the **type** and the **locale**:

### CLI Output

```bash
$ finetype infer -i "+33 1 42 68 53 00" --output json
{
  "label": "identity.person.phone_number",
  "locale": "FR",
  "broad_type": "VARCHAR"
}

$ finetype infer -i "M5V 3A8" --output json
{
  "label": "geography.address.postal_code",
  "locale": "EN_CA",
  "broad_type": "VARCHAR"
}
```

### Column Mode (Profile)

When profiling a CSV file, FineType detects locale per column:

```bash
$ finetype profile data.csv

Column: phone_numbers
  Type: identity.person.phone_number (98% confidence)
  Locale: US
  Sample values: +1 202 555 0100, +1 415 555 0100

Column: order_dates
  Type: datetime.date.iso_8601
  Locale: (not applicable — ISO 8601 is universal)
  Sample values: 2024-01-15, 2024-02-20
```

### DuckDB Extension

```sql
SELECT finetype_detail(phone_number) FROM customers;
-- Returns JSON with type, confidence, AND locale
-- → {"type":"identity.person.phone_number","confidence":0.98,"locale":"US"}

-- Filter by detected locale
SELECT customer_id, phone_number
FROM customers
WHERE json_extract_string(finetype_detail(phone_number), '$.locale') = 'DE';
```

## Locale-Specific Validation Examples

### Phone Numbers

Each locale has strict formatting rules:

| Region | Format | Example | Invalid Examples |
|--------|--------|---------|-------------------|
| **US** | (XXX) XXX-XXXX or +1 XXX XXX XXXX | (202) 555-0100 | +1 202-555-0100 (no parens) |
| **UK** | +44 20 XXXX XXXX or 020 XXXX XXXX | +44 20 7946 0958 | +44(0)207946 0958 (wrong paren format) |
| **DE** | +49 30 XXXXXXX or 030 XXXXXXX | +49 30 12345678 | 049 30 12345678 (no + sign) |
| **FR** | +33 1 XX XX XX XX or 01 XX XX XX XX | +33 1 42 68 53 00 | +33 142 685300 (wrong spacing) |
| **JP** | +81-XX-XXXX-XXXX or 09X-XXXX-XXXX | +81-90-1234-5678 | 090-1234-5678 (missing + for international) |

When you classify a value, FineType returns the matching locale:

```bash
$ finetype infer -i "(202) 555-0100"
identity.person.phone_number.EN_US

$ finetype infer -i "030 12345678"
identity.person.phone_number.DE

$ finetype infer -i "+33 1 42 68 53 00"
identity.person.phone_number.FR
```

### Postal Codes

65 locales, each with region-specific rules:

| Region | Format | Example | Notes |
|--------|--------|---------|-------|
| **US** | 5 digits or 9 digits (ZIP+4) | 90210 or 90210-1234 | Strict digit-only |
| **UK** | Alphanumeric, outward/inward code | SW1A 1AA, M1 1AE | Complex pattern rules |
| **CA** | Alphanumeric (ANA NAN) | M5V 3A8 | Alternating letters & digits |
| **DE** | 5 digits | 10115 | Simple numeric |
| **JP** | 7 digits (XXX-XXXX) | 100-0001 | Hyphen-separated |
| **BR** | 8 digits (XXXXX-XXX) | 01310-100 | Hyphen-separated |
| **IN** | 6 digits | 110001 | Numeric only |
| **AU** | 4 digits | 2000 | Numeric only |

FineType validates against all 65 patterns:

```bash
$ finetype infer -i "M5V 3A8"
geography.address.postal_code.EN_CA

$ finetype infer -i "10115"
geography.address.postal_code.DE

$ finetype infer -i "01310-100"
geography.address.postal_code.PT_BR
```

### Month Names

27 locales with full month names in that language:

```bash
$ finetype infer -i "janvier"
datetime.component.month_name.FR

$ finetype infer -i "Januar"
datetime.component.month_name.DE

$ finetype infer -i "enero"
datetime.component.month_name.ES

$ finetype infer -i "January"
datetime.component.month_name.EN
```

Useful for datasets with non-English date text:

```sql
-- French dataset
SELECT finetype(month_col) FROM french_dates;
-- → datetime.component.month_name.FR (not EN)

-- Then extract and transform
SELECT
  CASE finetype(month_col)
    WHEN 'datetime.component.month_name.FR' THEN parse_fr_month(month_col)
    WHEN 'datetime.component.month_name.EN' THEN strptime(month_col, '%B')
  END as month
FROM french_dates;
```

## Working with Locale Data

### Pattern 1: Filter by Locale

Extract phone numbers from a specific region:

```sql
-- DuckDB
SELECT * FROM customers
WHERE json_extract_string(finetype_detail(phone_number), '$.locale') = 'US';

-- CLI (in a script)
finetype infer -f phones.txt --output json \
  | jq 'select(.locale == "DE")' \
  | jq -r '.value'
```

### Pattern 2: Validate Consistency

Ensure all postal codes are from expected region(s):

```bash
# Generate report: are all postal codes from CA/US?
finetype profile data.csv --output json \
  | jq '.columns[] | select(.type == "geography.address.postal_code")'

# Output shows detected locale
# If locale is not CA/US, investigate data quality issue
```

### Pattern 3: Normalize by Locale

Convert locale-specific values to a canonical format:

```bash
# Generate synthetic samples with consistent locale
finetype generate --type geography.address.postal_code.CA --samples 100 > ca_postal_codes.txt

# Use as reference for validation
finetype infer -f customer_postal_codes.txt --output json \
  | jq 'select(.locale != "CA")' \
  | wc -l  # Count non-CA values
```

### Pattern 4: Internationalize Datasets

Classify a multi-country customer database:

```bash
finetype profile customers.csv \
  --output json > profile.json

# Extract locale info
jq '.columns[] | {name, detected_locale: .locale}' profile.json
# → {"name":"phone_number","detected_locale":"US"}
#   {"name":"address","detected_locale":"DE"}
#   {"name":"postal_code","detected_locale":"FR"}
```

## Locale Coverage Expansion

FineType continuously expands locale support. As of v0.5.3, the expansion roadmap includes:

### Phase 1: Core Types ✅ Complete
- ✅ Phone numbers (46 locales)
- ✅ Postal codes (65 locales)
- ✅ Month/day names (27 locales each)

### Phase 2: Address Types (Planned)
- Full addresses (street ordering, region-specific rules)
- Street names (local naming conventions)
- City names (region validation)

### Phase 3: Extended Types (Future)
- Person names (Unicode patterns per locale)
- Currency formats (symbol position, thousand separators)
- Date formats (CLDR expansion to 50+ locales)

## Locales by Region

### Western Europe (14 locales)
- 🇬🇧 **GB** (England, phone, postal, month, day)
- 🇫🇷 **FR** (France, phone, postal, month, day)
- 🇩🇪 **DE** (Germany, phone, postal, month, day)
- 🇪🇸 **ES** (Spain, phone, month, day)
- 🇮🇹 **IT** (Italy, phone, month, day)
- 🇳🇱 **NL** (Netherlands, phone, postal)
- 🇵🇱 **PL** (Poland, phone, postal, month, day)
- 🇵🇹 **PT** (Portugal, month, day)
- 🇧🇪 **BE** (Belgium, postal)
- 🇦🇹 **AT** (Austria, postal)
- 🇨🇭 **CH** (Switzerland, postal)
- 🇸🇪 **SE** (Sweden, postal, month, day)
- 🇳🇴 **NO** (Norway, postal, month, day)
- 🇫🇮 **FI** (Finland, postal, month, day)

### Eastern Europe (12 locales)
- 🇷🇺 **RU** (Russia, phone, postal, month, day)
- 🇷🇴 **RO** (Romania, postal, month, day)
- 🇭🇺 **HU** (Hungary, postal, month, day)
- 🇨🇿 **CZ** (Czech Republic, postal, month, day)
- 🇸🇰 **SK** (Slovakia, postal, month, day)
- 🇧🇬 **BG** (Bulgaria, postal, month, day)
- 🇭🇷 **HR** (Croatia, postal, month, day)
- 🇸🇮 **SI** (Slovenia, postal, month, day)
- 🇺🇦 **UA** (Ukraine, month, day)
- 🇱🇹 **LT** (Lithuania, postal, month, day)
- 🇱🇻 **LV** (Latvia, postal, month, day)
- 🇪🇪 **EE** (Estonia, postal, month, day)

### Asia-Pacific (12 locales)
- 🇯🇵 **JA** (Japan, phone, postal, calling code)
- 🇰🇷 **KO** (South Korea, phone, calling code)
- 🇨🇳 **ZH** (China, phone, calling code)
- 🇦🇺 **AU** (Australia, phone, postal, calling code)
- 🇮🇩 **ID** (Indonesia, postal)
- 🇵🇭 **PH** (Philippines, postal)
- 🇹🇭 **TH** (Thailand, postal)
- 🇲🇾 **MY** (Malaysia, postal)
- 🇸🇬 **SG** (Singapore, postal)
- 🇳🇿 **NZ** (New Zealand)
- 🇹🇼 **TW** (Taiwan, postal)
- 🇭🇰 **HK** (Hong Kong, postal)

### Americas (8 locales)
- 🇺🇸 **US** (United States, phone, postal, calling code)
- 🇨🇦 **CA** (Canada, phone, postal, calling code)
- 🇲🇽 **MX** (Mexico, postal)
- 🇧🇷 **BR** (Brazil, postal)
- 🇦🇷 **AR** (Argentina, postal, month, day)
- 🇨🇱 **CL** (Chile, postal)
- 🇵🇪 **PE** (Peru, postal)
- 🇨🇴 **CO** (Colombia, postal)

### Africa & Middle East (8 locales)
- 🇿🇦 **ZA** (South Africa, phone, postal, calling code)
- 🇳🇬 **NG** (Nigeria, postal)
- 🇪🇬 **EG** (Egypt, postal, month, day)
- 🇮🇳 **IN** (India, postal)
- 🇮🇱 **IL** (Israel, postal, calling code)
- 🇬🇷 **GR** (Greece, postal, month, day)
- 🇹🇷 **TR** (Turkey, postal, month, day)
- 🇸🇦 **SA** (Saudi Arabia, postal, month, day)

## Troubleshooting Locale Detection

### "FineType returns wrong locale"

This can happen when data is ambiguous or doesn't match expected patterns. Examples:

```bash
# A phone number that looks like US but is actually GB
$ finetype infer -i "020 7946 0958"
# Returns: identity.person.phone_number.US
# Reason: 10-digit number matches both US and GB patterns
# Solution: Provide +44 prefix if available, or check region-specific validation rules

# A postal code that's ambiguous
$ finetype infer -i "10115"
# Returns: geography.address.postal_code.DE
# Could also be valid in other regions
# Solution: Include region context (headers, related columns) for disambiguation
```

**How to fix:**
1. Check column headers — FineType uses header text for disambiguation
2. Provide full international format (e.g., +33 for France, +49 for Germany)
3. Use column context — if other columns confirm region, use Sense & Sharpen mode (`finetype profile --mode column`)

### "All my postal codes show as different locales"

This usually means the data is genuinely multi-region:

```bash
# Example: a global retailer with postal codes from 15 countries
finetype profile customers.csv

# Output shows mixed locales per column:
# postal_code: detected locales: US (600), CA (200), GB (100), FR (50), ...
```

**What to do:**
- This is correct behavior — your data *is* multi-locale
- Use `finetype_detail()` in DuckDB to separate and transform by locale
- Or filter/validate by expected region using locale information

## Advanced: Using Locale in ETL Pipelines

### Example: Normalize International Phone Numbers

```bash
#!/bin/bash
# Input: customers.csv with phone_number column (mixed formats)

# Step 1: Detect locales
finetype profile customers.csv --output json > profile.json

# Step 2: Extract phone locale
phone_locale=$(jq -r '.columns[] | select(.name == "phone_number") | .locale' profile.json)

# Step 3: Transform based on locale
case $phone_locale in
  US) sed 's/^\+1//' customers.csv > normalized.csv ;;
  GB) sed 's/^0/+440/' customers.csv > normalized.csv ;;
  DE) sed 's/^0/+49/' customers.csv > normalized.csv ;;
  *) cp customers.csv normalized.csv ;;
esac

echo "Normalized to locale: $phone_locale"
```

### Example: Validate Multi-Region Dataset

```sql
-- DuckDB: Validate postal codes match expected regions
SELECT
  region,
  COUNT(*) as postal_codes,
  COUNT(CASE
    WHEN json_extract_string(finetype_detail(postal_code), '$.locale') = region
    THEN 1
  END) as validated
FROM orders
GROUP BY region
ORDER BY validated DESC;

-- Example output:
-- region | postal_codes | validated
-- -------|------|----------
-- US     | 5000 | 4998
-- CA     | 1000 | 998
-- DE     | 500  | 495
-- FR     | 300  | 298
-- ...
```

## Learning More

- **LOCALE_DETECTION_ARCHITECTURE.md** — Why FineType uses post-hoc validation instead of model-based locale classification
- **TAXONOMY_COMPARISON.md** — How FineType's locale system compares to other data typing systems
- **CLI Reference** — `finetype --help` for locale-related flags
- **DuckDB Functions** — `SELECT finetype_version()` to see supported locales
