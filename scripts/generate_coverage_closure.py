#!/usr/bin/env python3
"""Generate eval/datasets/csv/coverage_closure_phase_ab.csv and matching
manifest rows for every Phase A+B zero-coverage type (ac-05).

Per MADR 0057, Phase A+B closes zero-coverage only — ≥1 column per type
with ≥5 non-null values. Edge-case second-column coverage is Phase C
(out of scope for this spec).

Values are hand-curated to realistic format (no LLM-as-judge for
sourcing — constraint #1). Each column gets 5-8 values. For types in the
restricted-registry carve-out (MADR 0055), values are synthetic-necessary
with explicit rationale in the carve-out table.

Usage:
    python scripts/generate_coverage_closure.py --write
"""

from __future__ import annotations

import argparse
import csv
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# Column values per missing taxonomy type. Leaf names are used as column
# headers. gt_label for the manifest row is the FULL taxonomy type — this
# is the direct-match path (resolve_coverage rule 2), bypassing schema_mapping
# so the closure is self-describing.
COVERAGE: dict[str, list[str]] = {
    # ─── container ──────────────────────────────────────────────
    "container.array.comma_separated": [
        "apple,banana,cherry", "red,green,blue,yellow", "1,2,3,4,5",
        "dog,cat,bird", "jan,feb,mar,apr", "alpha,beta,gamma",
    ],
    "container.array.pipe_separated": [
        "apple|banana|cherry", "red|green|blue", "1|2|3|4|5",
        "dog|cat|bird", "jan|feb|mar", "alpha|beta|gamma",
    ],
    "container.array.semicolon_separated": [
        "apple;banana;cherry", "red;green;blue", "1;2;3;4;5",
        "dog;cat;bird", "jan;feb;mar", "alpha;beta;gamma",
    ],
    "container.array.whitespace_separated": [
        "apple banana cherry", "red green blue", "1 2 3 4 5",
        "dog cat bird", "jan feb mar", "alpha beta gamma",
    ],
    "container.key_value.query_string": [
        "name=alice&age=30", "lat=51.5&lng=-0.1", "q=rust&lang=en",
        "id=42&type=user", "page=1&size=20", "sort=asc&filter=active",
    ],
    "container.object.csv": [
        "a,b,c\n1,2,3", "name,age\nalice,30\nbob,25", "x,y\n0.1,0.2\n0.3,0.4",
        "id,status\n1,active\n2,idle", "k,v\nfoo,1\nbar,2",
        "a,b\nx,y\np,q",
    ],
    "container.object.html": [
        "<p>hello</p>", "<div><span>world</span></div>",
        "<a href='x'>link</a>", "<h1>Title</h1><p>body</p>",
        "<ul><li>one</li><li>two</li></ul>", "<br/><hr/>",
    ],
    "container.object.json_array": [
        "[1,2,3]", '["a","b","c"]', "[{\"k\":1},{\"k\":2}]",
        "[true,false,null]", "[[1,2],[3,4]]", "[\"x\",0.5,null]",
    ],
    "container.object.xml": [
        "<x>1</x>", "<root><a>v</a></root>", "<item id='1'>hello</item>",
        "<list><e>1</e><e>2</e></list>", "<n>42</n>",
        "<r><a>1</a><b>2</b></r>",
    ],
    "container.object.yaml": [
        "key: value", "a:\n  b: 1\n  c: 2", "- item1\n- item2",
        "name: alice\nage: 30", "x: [1,2,3]", "flag: true",
    ],

    # ─── datetime.component ─────────────────────────────────────
    "datetime.component.periodicity": [
        "daily", "weekly", "monthly", "quarterly", "yearly", "annual",
    ],

    # ─── datetime.date (variants) ───────────────────────────────
    "datetime.date.abbrev_month_no_comma": [
        "Jan 15 2024", "Mar 22 2023", "Dec 25 2022", "Jul 04 2024",
        "Nov 11 2023", "Aug 19 2024",
    ],
    "datetime.date.chinese_ymd": [
        "2024年03月15日", "2023年12月25日", "2022年01月01日",
        "2024年07月04日", "2023年11月11日", "2024年08月19日",
    ],
    "datetime.date.compact_mdy": [
        "03152024", "12252023", "01012022", "07042024",
        "11112023", "08192024",
    ],
    "datetime.date.compact_ym": [
        "202403", "202312", "202201", "202407", "202311", "202408",
    ],
    "datetime.date.dmy_dash_abbrev": [
        "15-Mar-2024", "25-Dec-2023", "01-Jan-2022", "04-Jul-2024",
        "11-Nov-2023", "19-Aug-2024",
    ],
    "datetime.date.dmy_dash_abbrev_short": [
        "15-Mar-24", "25-Dec-23", "01-Jan-22", "04-Jul-24",
        "11-Nov-23", "19-Aug-24",
    ],
    "datetime.date.dmy_short_dot": [
        "15.03.24", "25.12.23", "01.01.22", "04.07.24",
        "11.11.23", "19.08.24",
    ],
    "datetime.date.dmy_short_slash": [
        "15/03/24", "25/12/23", "01/01/22", "04/07/24",
        "11/11/23", "19/08/24",
    ],
    "datetime.date.dmy_space_abbrev": [
        "15 Mar 2024", "25 Dec 2023", "01 Jan 2022", "04 Jul 2024",
        "11 Nov 2023", "19 Aug 2024",
    ],
    "datetime.date.dmy_space_full": [
        "15 March 2024", "25 December 2023", "01 January 2022",
        "04 July 2024", "11 November 2023", "19 August 2024",
    ],
    "datetime.date.full_month_no_comma": [
        "March 15 2024", "December 25 2023", "January 01 2022",
        "July 04 2024", "November 11 2023", "August 19 2024",
    ],
    "datetime.date.jp_era_long": [
        "令和06年03月15日", "令和05年12月25日", "平成34年01月01日",
        "令和06年07月04日", "令和05年11月11日", "令和06年08月19日",
    ],
    "datetime.date.jp_era_short": [
        "R06.03.15", "R05.12.25", "H34.01.01",
        "R06.07.04", "R05.11.11", "R06.08.19",
    ],
    "datetime.date.julian": [
        "2024075", "2023359", "2022001", "2024186", "2023315", "2024232",
    ],
    "datetime.date.korean_ymd": [
        "2024년 03월 15일", "2023년 12월 25일", "2022년 01월 01일",
        "2024년 07월 04일", "2023년 11월 11일", "2024년 08월 19일",
    ],
    "datetime.date.mdy_short_slash": [
        "03/15/24", "12/25/23", "01/01/22", "07/04/24",
        "11/11/23", "08/19/24",
    ],
    "datetime.date.month_year_abbrev": [
        "Mar 2024", "Dec 2023", "Jan 2022", "Jul 2024", "Nov 2023", "Aug 2024",
    ],
    "datetime.date.month_year_full": [
        "March 2024", "December 2023", "January 2022",
        "July 2024", "November 2023", "August 2024",
    ],
    "datetime.date.month_year_slash": [
        "03/2024", "12/2023", "01/2022", "07/2024", "11/2023", "08/2024",
    ],
    "datetime.date.ordinal": [
        "March 15th, 2024", "December 25th, 2023", "January 1st, 2022",
        "July 4th, 2024", "November 11th, 2023", "August 19th, 2024",
    ],
    "datetime.date.short_dmy": [
        "15/3/2024", "25/12/2023", "1/1/2022", "4/7/2024", "11/11/2023", "19/8/2024",
    ],
    "datetime.date.short_mdy": [
        "3/15/2024", "12/25/2023", "1/1/2022", "7/4/2024", "11/11/2023", "8/19/2024",
    ],
    "datetime.date.short_ymd": [
        "2024/3/15", "2023/12/25", "2022/1/1", "2024/7/4", "2023/11/11", "2024/8/19",
    ],
    "datetime.date.weekday_abbreviated_month": [
        "Fri, Mar 15, 2024", "Mon, Dec 25, 2023", "Sat, Jan 01, 2022",
        "Thu, Jul 04, 2024", "Sat, Nov 11, 2023", "Mon, Aug 19, 2024",
    ],
    "datetime.date.weekday_dmy_full": [
        "Friday, 15 March 2024", "Monday, 25 December 2023",
        "Saturday, 01 January 2022", "Thursday, 04 July 2024",
        "Saturday, 11 November 2023", "Monday, 19 August 2024",
    ],
    "datetime.date.weekday_full_month": [
        "Friday, March 15, 2024", "Monday, December 25, 2023",
        "Saturday, January 01, 2022", "Thursday, July 04, 2024",
        "Saturday, November 11, 2023", "Monday, August 19, 2024",
    ],
    "datetime.date.year_month": [
        "2024-03", "2023-12", "2022-01", "2024-07", "2023-11", "2024-08",
    ],
    "datetime.date.ymd_slash": [
        "2024/03/15", "2023/12/25", "2022/01/01",
        "2024/07/04", "2023/11/11", "2024/08/19",
    ],

    # ─── datetime.epoch ────────────────────────────────────────
    "datetime.epoch.unix_seconds": [
        "1710504000", "1703462400", "1640995200",
        "1720051200", "1699660800", "1724025600",
    ],
    "datetime.epoch.unix_milliseconds": [
        "1710504000000", "1703462400000", "1640995200000",
        "1720051200000", "1699660800000", "1724025600000",
    ],
    "datetime.epoch.unix_microseconds": [
        "1710504000000000", "1703462400000000", "1640995200000000",
        "1720051200000000", "1699660800000000", "1724025600000000",
    ],

    # ─── datetime.time ─────────────────────────────────────────
    "datetime.time.hm_24h": [
        "09:30", "14:45", "23:59", "06:00", "12:00", "17:15",
    ],
    "datetime.time.hms_24h": [
        "09:30:15", "14:45:22", "23:59:59", "06:00:00", "12:00:00", "17:15:30",
    ],
    "datetime.time.iso": [
        "09:30:15Z", "14:45:22+02:00", "23:59:59-05:00",
        "06:00:00Z", "12:00:00+09:00", "17:15:30-03:00",
    ],

    # ─── datetime.timestamp ────────────────────────────────────
    "datetime.timestamp.ctime": [
        "Fri Mar 15 09:30:15 2024", "Mon Dec 25 14:45:22 2023",
        "Sat Jan 01 00:00:00 2022", "Thu Jul 04 12:00:00 2024",
        "Sat Nov 11 17:15:30 2023", "Mon Aug 19 08:00:00 2024",
    ],
    "datetime.timestamp.dot_dmy_24h": [
        "15.03.2024 09:30", "25.12.2023 14:45", "01.01.2022 00:00",
        "04.07.2024 12:00", "11.11.2023 17:15", "19.08.2024 08:00",
    ],
    "datetime.timestamp.dot_ymd_24h": [
        "2024.03.15 09:30", "2023.12.25 14:45", "2022.01.01 00:00",
        "2024.07.04 12:00", "2023.11.11 17:15", "2024.08.19 08:00",
    ],
    "datetime.timestamp.epoch_nanoseconds": [
        "1710504000000000000", "1703462400000000000", "1640995200000000000",
        "1720051200000000000", "1699660800000000000", "1724025600000000000",
    ],
    "datetime.timestamp.iso_8601_compact": [
        "20240315T093015Z", "20231225T144522Z", "20220101T000000Z",
        "20240704T120000Z", "20231111T171530Z", "20240819T080000Z",
    ],
    "datetime.timestamp.iso_8601_microseconds": [
        "2024-03-15T09:30:15.123456", "2023-12-25T14:45:22.987654",
        "2022-01-01T00:00:00.000001", "2024-07-04T12:00:00.500000",
        "2023-11-11T17:15:30.250000", "2024-08-19T08:00:00.750000",
    ],
    "datetime.timestamp.iso_8601_micros_offset": [
        "2024-03-15T09:30:15.123456+00:00", "2023-12-25T14:45:22.987654+02:00",
        "2022-01-01T00:00:00.000001-05:00", "2024-07-04T12:00:00.500000Z",
        "2023-11-11T17:15:30.250000+09:00", "2024-08-19T08:00:00.750000-03:00",
    ],
    "datetime.timestamp.iso_8601_milliseconds": [
        "2024-03-15T09:30:15.123", "2023-12-25T14:45:22.987",
        "2022-01-01T00:00:00.000", "2024-07-04T12:00:00.500",
        "2023-11-11T17:15:30.250", "2024-08-19T08:00:00.750",
    ],
    "datetime.timestamp.iso_8601_millis_offset": [
        "2024-03-15T09:30:15.123+00:00", "2023-12-25T14:45:22.987+02:00",
        "2022-01-01T00:00:00.000-05:00", "2024-07-04T12:00:00.500Z",
        "2023-11-11T17:15:30.250+09:00", "2024-08-19T08:00:00.750-03:00",
    ],
    "datetime.timestamp.iso_microseconds": [
        "2024-03-15 09:30:15.123456", "2023-12-25 14:45:22.987654",
        "2022-01-01 00:00:00.000001", "2024-07-04 12:00:00.500000",
        "2023-11-11 17:15:30.250000", "2024-08-19 08:00:00.750000",
    ],
    "datetime.timestamp.iso_space_zulu": [
        "2024-03-15 09:30:15Z", "2023-12-25 14:45:22Z", "2022-01-01 00:00:00Z",
        "2024-07-04 12:00:00Z", "2023-11-11 17:15:30Z", "2024-08-19 08:00:00Z",
    ],
    "datetime.timestamp.mdy_24h": [
        "03/15/2024 09:30:15", "12/25/2023 14:45:22", "01/01/2022 00:00:00",
        "07/04/2024 12:00:00", "11/11/2023 17:15:30", "08/19/2024 08:00:00",
    ],
    "datetime.timestamp.pg_short_offset": [
        "2024-03-15 09:30:15+00", "2023-12-25 14:45:22+02",
        "2022-01-01 00:00:00-05", "2024-07-04 12:00:00+00",
        "2023-11-11 17:15:30+09", "2024-08-19 08:00:00-03",
    ],
    "datetime.timestamp.rfc_2822_ordinal": [
        "Fri, 15th Mar 2024 09:30:15 +0000", "Mon, 25th Dec 2023 14:45:22 +0000",
        "Sat, 1st Jan 2022 00:00:00 +0000", "Thu, 4th Jul 2024 12:00:00 +0000",
        "Sat, 11th Nov 2023 17:15:30 +0000", "Mon, 19th Aug 2024 08:00:00 +0000",
    ],
    "datetime.timestamp.slash_ymd_24h": [
        "2024/03/15 09:30:15", "2023/12/25 14:45:22", "2022/01/01 00:00:00",
        "2024/07/04 12:00:00", "2023/11/11 17:15:30", "2024/08/19 08:00:00",
    ],
    "datetime.timestamp.sql_microseconds": [
        "2024-03-15 09:30:15.123456", "2023-12-25 14:45:22.987654",
        "2022-01-01 00:00:00.000001", "2024-07-04 12:00:00.500000",
        "2023-11-11 17:15:30.250000", "2024-08-19 08:00:00.750000",
    ],
    "datetime.timestamp.sql_microseconds_offset": [
        "2024-03-15 09:30:15.123456+00:00", "2023-12-25 14:45:22.987654+02:00",
        "2022-01-01 00:00:00.000001-05:00", "2024-07-04 12:00:00.500000+00:00",
        "2023-11-11 17:15:30.250000+09:00", "2024-08-19 08:00:00.750000-03:00",
    ],
    "datetime.timestamp.sql_milliseconds": [
        "2024-03-15 09:30:15.123", "2023-12-25 14:45:22.987",
        "2022-01-01 00:00:00.000", "2024-07-04 12:00:00.500",
        "2023-11-11 17:15:30.250", "2024-08-19 08:00:00.750",
    ],

    # ─── finance.crypto / currency / rate / securities ──────────
    "finance.crypto.ethereum_address": [
        "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8",
        "0xAb5801a7D398351b8bE11C439e05C5B3259aeC9B",
        "0xE853c56864A2ebe4576a807D26Fdc4A0adA51919",
        "0x220866B1A2219f40e72f5c628B65D54268cA3A9D",
        "0x0000000000000000000000000000000000000001",
        "0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe",
    ],
    "finance.currency.amount": [
        "$1,234.56", "$99.99", "€500.00", "£42.75", "¥1200", "$0.01",
    ],
    "finance.currency.amount_accounting": [
        "(1,234.56)", "(99.99)", "1,234.56", "99.99", "(0.01)", "42.75",
    ],
    "finance.currency.amount_apostrophe": [
        "1'234.56", "99'999.00", "12'345.67", "1'000'000.00",
        "500.00", "42.75",
    ],
    "finance.currency.amount_code_prefix": [
        "USD 1234.56", "EUR 99.99", "GBP 42.75", "JPY 1200",
        "CHF 500.00", "CAD 250.00",
    ],
    "finance.currency.amount_comma": [
        "1,234.56", "99,999.00", "12,345.67", "1,000,000.00",
        "500.00", "42.75",
    ],
    "finance.currency.amount_comma_suffix": [
        "1234,56", "99,99", "500,00", "42,75", "1000,00", "12345,67",
    ],
    "finance.currency.amount_crypto": [
        "0.00123456 BTC", "1.5 ETH", "100 USDT", "0.5 BTC",
        "2.25 ETH", "50 USDC",
    ],
    "finance.currency.amount_lakh": [
        "1,23,456.78", "10,00,000.00", "5,50,000.00", "25,00,000.00",
        "1,50,000.00", "99,99,999.00",
    ],
    "finance.currency.amount_multisym": [
        "$€1,234.56", "$£99.99", "€£500.00", "¥$42",
        "$CAD 100", "EUR€ 250",
    ],
    "finance.currency.amount_neg_trailing": [
        "1,234.56-", "99.99-", "500.00-", "42.75-", "0.01-", "1000.00-",
    ],
    "finance.currency.amount_nodecimal": [
        "$1234", "€99", "£42", "¥1200", "$500", "$0",
    ],
    "finance.currency.amount_space": [
        "1 234.56", "99 999.00", "12 345.67", "1 000 000.00",
        "500.00", "42.75",
    ],
    "finance.rate.basis_points": [
        "25 bps", "100 bps", "-50 bps", "250 bps", "10 bps", "500 bps",
    ],
    "finance.rate.yield": [
        "3.25%", "4.50%", "2.75%", "5.125%", "1.99%", "6.75%",
    ],
    "finance.securities.sedol": [
        "B0YBKJ7", "2046251", "0263494", "B0YBKL9",
        "B1VZ0M2", "2073373",
    ],

    # ─── geography ──────────────────────────────────────────────
    "geography.address.street_name": [
        "Main Street", "Oak Avenue", "Elm Road", "Park Lane",
        "First Avenue", "Broadway",
    ],
    "geography.address.street_suffix": [
        "Street", "Avenue", "Boulevard", "Road", "Lane", "Drive",
    ],
    "geography.contact.calling_code": [
        "+1", "+44", "+81", "+49", "+33", "+86",
    ],
    "geography.location.continent": [
        "North America", "Europe", "Asia", "South America", "Africa", "Oceania",
    ],
    "geography.location.state_code": [
        "CA", "NY", "TX", "FL", "WA", "IL",
    ],

    # ─── identity ───────────────────────────────────────────────
    "identity.medical.dea_number": [
        "AB1234567", "BK9876543", "FC1111111", "MG2222222",
        "AS3333333", "BZ4444444",
    ],
    "identity.medical.ndc": [
        "0002-7510-01", "0777-3105-02", "50580-449-03",
        "68180-517-07", "0378-0208-01", "51079-952-20",
    ],
    "identity.person.blood_type": [
        "A+", "O-", "B+", "AB+", "O+", "A-",
    ],
    "identity.person.gender_code": [
        "M", "F", "U", "N", "M", "F",
    ],
    "identity.person.password": [
        "SecureP@ss1", "MyStr0ng!PW", "C0mpl3x#Pw",
        "R@nd0mP@ss2024", "H@rd_to_Gu3ss", "V3ry$ecure!",
    ],

    # ─── representation ─────────────────────────────────────────
    "representation.boolean.initials": [
        "Y", "N", "Y", "N", "Y", "N",
    ],
    "representation.boolean.terms": [
        "yes", "no", "true", "false", "on", "off",
    ],
    "representation.discrete.ordinal": [
        "1st", "2nd", "3rd", "4th", "5th", "10th",
    ],
    "representation.file.excel_format": [
        "xlsx", "xls", "csv", "ods", "xlsm", "xlsb",
    ],
    "representation.file.extension": [
        ".pdf", ".docx", ".txt", ".jpg", ".png", ".zip",
    ],
    "representation.format.color_rgb": [
        "rgb(255, 0, 0)", "rgb(0, 255, 0)", "rgb(0, 0, 255)",
        "rgb(128, 128, 128)", "rgb(255, 255, 0)", "rgb(255, 165, 0)",
    ],
    "representation.identifier.numeric_code": [
        "001", "042", "123", "999", "500", "007",
    ],
    "representation.numeric.si_number": [
        "1k", "2.5M", "750m", "1.2µ", "3.14G", "500n",
    ],
    "representation.scientific.dna_sequence": [
        "ATCGATCGATCG", "GGCATTAACG", "CCGGAATT",
        "AAATTTCCCGGG", "ATGAAACCCTTT", "GCATGCATGC",
    ],
    "representation.scientific.measurement_unit": [
        "kg", "m/s", "°C", "mmHg", "Pa", "μm",
    ],
    "representation.scientific.protein_sequence": [
        "MKVLWAALLVTFL", "MALWMRLLPLLAL", "MQIFVKTLTGK",
        "ACDEFGHIKLMNPQRSTVWY", "MELVTKLAA", "GGGGSGGGGS",
    ],
    "representation.scientific.rna_sequence": [
        "AUCGAUCGAUCG", "GGCAUUAACG", "CCGGAAUU",
        "AAAUUUCCCGGG", "AUGAAACCCUUU", "GCAUGCAUGC",
    ],
    "representation.text.emoji": [
        "😀", "🎉", "🚀", "❤️", "👍", "🌟",
    ],
    "representation.text.plain_text": [
        "This is a sentence of plain English text.",
        "The quick brown fox jumps over the lazy dog.",
        "Data analysis is important for decision making.",
        "Customer feedback provides valuable insights.",
        "Machine learning models require quality data.",
        "Clear communication saves everyone time.",
    ],
    "representation.text.word": [
        "apple", "banana", "cherry", "dolphin", "elephant", "forest",
    ],

    # ─── technology ─────────────────────────────────────────────
    "technology.code.doi": [
        "10.1038/nature12373", "10.1126/science.1259855",
        "10.1145/3290605.3300563", "10.1109/ACCESS.2020.1234567",
        "10.1016/j.cell.2019.04.001", "10.1007/s00125-020-05178-5",
    ],
    "technology.code.imei": [
        "490154203237518", "352099001761481", "353918070033547",
        "356938035643809", "490154203237526", "352099001761499",
    ],
    "technology.cryptographic.token_urlsafe": [
        "abc123XYZ-_456def", "QWERTYasdf-_1234",
        "tokenA-tokenB-_xyz", "jwt.header.payload_sig",
        "K-a1b2c3_xYz-9", "_-authTokenXyZ123",
    ],
    "technology.internet.http_method": [
        "GET", "POST", "PUT", "DELETE", "PATCH", "HEAD",
    ],
    "technology.internet.ip_v4_with_port": [
        "192.168.1.1:80", "10.0.0.1:443", "172.16.0.1:22",
        "8.8.8.8:53", "127.0.0.1:5432", "192.0.2.1:8080",
    ],
    "technology.internet.top_level_domain": [
        "com", "org", "net", "io", "co.uk", "dev",
    ],
}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0] if __doc__ else "")
    parser.add_argument("--write", action="store_true", help="Actually write files.")
    parser.add_argument(
        "--output-csv",
        type=Path,
        default=REPO_ROOT / "eval/datasets/csv/coverage_closure_phase_ab.csv",
    )
    parser.add_argument(
        "--manifest",
        type=Path,
        default=REPO_ROOT / "eval/datasets/manifest.csv",
    )
    args = parser.parse_args()

    # Build wide CSV: each taxonomy type is a column, leaf name is header.
    # Different columns have different lengths — pad to max with empty strings.
    types = sorted(COVERAGE.keys())
    leaves = [t.rsplit(".", 1)[-1] for t in types]

    # Detect leaf collisions (e.g., two types sharing the same last segment)
    seen: dict[str, str] = {}
    dedup_leaves: list[str] = []
    for t, leaf in zip(types, leaves):
        if leaf in seen:
            # Disambiguate by using category.leaf
            parts = t.split(".")
            leaf = f"{parts[-2]}_{parts[-1]}"
        seen[leaf] = t
        dedup_leaves.append(leaf)

    max_len = max(len(COVERAGE[t]) for t in types)
    rows: list[list[str]] = []
    for i in range(max_len):
        row = []
        for t in types:
            vals = COVERAGE[t]
            row.append(vals[i] if i < len(vals) else "")
        rows.append(row)

    print(f"Coverage closure: {len(types)} types, {max_len} data rows")

    if not args.write:
        print("(dry-run — pass --write to save)")
        return 0

    args.output_csv.parent.mkdir(parents=True, exist_ok=True)
    with open(args.output_csv, "w", newline="", encoding="utf-8") as fh:
        w = csv.writer(fh)
        w.writerow(dedup_leaves)
        w.writerows(rows)
    print(f"wrote {args.output_csv}")

    # Append manifest rows
    rel_path = args.output_csv.relative_to(REPO_ROOT).as_posix()
    source_url = f"repo://{rel_path}"
    licence = "internal"
    fetched = "2026-04-21"
    new_rows = []
    for t, leaf in zip(types, dedup_leaves):
        new_rows.append({
            "dataset": "coverage_closure_phase_ab",
            "file_path": rel_path,
            "column_name": leaf,
            "gt_label": t,  # full taxonomy type — self-describing
            "source_url": source_url,
            "licence": licence,
            "fetched_date": fetched,
        })

    # Read existing manifest
    with open(args.manifest, newline="", encoding="utf-8") as fh:
        reader = csv.DictReader(fh)
        existing = list(reader)
        fieldnames = list(reader.fieldnames or [])

    # Dedupe: skip any row already present (dataset, column_name)
    existing_keys = {(r["dataset"], r["column_name"]) for r in existing}
    add = [r for r in new_rows if (r["dataset"], r["column_name"]) not in existing_keys]

    with open(args.manifest, "w", newline="", encoding="utf-8") as fh:
        w = csv.DictWriter(fh, fieldnames=fieldnames)
        w.writeheader()
        w.writerows(existing)
        w.writerows(add)

    print(f"appended {len(add)} manifest rows ({len(new_rows) - len(add)} duplicates skipped)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
