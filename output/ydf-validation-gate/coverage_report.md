# YDF validation-gate coverage report

Per spec `2026-05-26-ydf-validation-gate` ac-03.

For each YDF predicted label, this report shows the count of predictions made vs refused by the gate, across the four tracked corpus passes (v19 through v22). Refusal rate is `refused / (refused + kept)`. The gate refuses a prediction when fewer than 50% of the column's sampled values pass the label's JSON Schema validation (per ac-01).

## v19

**5,998,275 predictions** — 5,813,522 kept, 184,753 refused (3.08% refusal rate).

### Top 25 by refusal count

| YDF label | kept | refused | total | refusal % |
|---|---:|---:|---:|---:|
| **datetime.component.year** | 18,053 | **40,533** | 58,586 | 69.2% |
| **representation.numeric.decimal_number_comma** | 1,263 | **20,246** | 21,509 | 94.1% |
| **identity.commerce.isbn** | 1,999 | **18,748** | 20,747 | 90.4% |
| representation.numeric.integer_number | 1,260,689 | 16,955 | 1,277,644 | 1.3% |
| **datetime.time.hm_24h** | 10,779 | **12,002** | 22,781 | 52.7% |
| representation.numeric.decimal_number | 1,791,595 | 7,404 | 1,798,999 | 0.4% |
| representation.identifier.alphanumeric_id | 47,949 | 5,702 | 53,651 | 10.6% |
| **finance.payment.credit_card_number** | 194 | **5,117** | 5,311 | 96.3% |
| **identity.person.phone_e164** | 0 | **4,690** | 4,690 | 100.0% |
| **finance.banking.aba_routing** | 1,861 | **4,070** | 5,931 | 68.6% |
| **datetime.date.compact_ym** | 220 | **3,823** | 4,043 | 94.6% |
| **geography.transportation.iso6346** | 0 | **3,388** | 3,388 | 100.0% |
| **datetime.timestamp.slash_ymd_24h** | 81 | **3,327** | 3,408 | 97.6% |
| **datetime.timestamp.epoch_nanoseconds** | 39 | **3,213** | 3,252 | 98.8% |
| **technology.identifier.snowflake_id** | 34 | **2,825** | 2,859 | 98.8% |
| **datetime.time.hms_24h** | 395 | **2,660** | 3,055 | 87.1% |
| **finance.crypto.ethereum_address** | 79 | **2,532** | 2,611 | 97.0% |
| representation.numeric.percentage | 8,794 | 2,096 | 10,890 | 19.2% |
| datetime.date.mdy_slash | 6,824 | 1,998 | 8,822 | 22.6% |
| geography.location.country_code | 2,309 | 1,729 | 4,038 | 42.8% |
| datetime.epoch.unix_seconds | 76,527 | 1,728 | 78,255 | 2.2% |
| **datetime.timestamp.rfc_3339** | 62 | **1,398** | 1,460 | 95.8% |
| **finance.banking.iban** | 40 | **1,333** | 1,373 | 97.1% |
| **identity.government.ein** | 9 | **1,169** | 1,178 | 99.2% |
| **datetime.epoch.unix_microseconds** | 36 | **1,142** | 1,178 | 96.9% |

### Canary types (expect high refusal)

| YDF label | kept | refused | total | refusal % |
|---|---:|---:|---:|---:|
| geography.transportation.iso6346 | 0 | 3,388 | 3,388 | 100.0% |
| geography.coordinate.mgrs | 0 | 38 | 38 | 100.0% |
| geography.coordinate.plus_code | 0 | 4 | 4 | 100.0% |
| finance.payment.credit_card_number | 194 | 5,117 | 5,311 | 96.3% |
| identity.person.phone_e164 | 0 | 4,690 | 4,690 | 100.0% |

## v20

**5,994,768 predictions** — 5,809,996 kept, 184,772 refused (3.08% refusal rate).

### Top 25 by refusal count

| YDF label | kept | refused | total | refusal % |
|---|---:|---:|---:|---:|
| **datetime.component.year** | 18,034 | **40,524** | 58,558 | 69.2% |
| **representation.numeric.decimal_number_comma** | 1,261 | **20,093** | 21,354 | 94.1% |
| **identity.commerce.isbn** | 1,865 | **18,933** | 20,798 | 91.0% |
| representation.numeric.integer_number | 1,260,428 | 16,794 | 1,277,222 | 1.3% |
| **datetime.time.hm_24h** | 10,781 | **12,002** | 22,783 | 52.7% |
| representation.numeric.decimal_number | 1,792,975 | 7,409 | 1,800,384 | 0.4% |
| representation.identifier.alphanumeric_id | 47,256 | 5,672 | 52,928 | 10.7% |
| **finance.payment.credit_card_number** | 221 | **5,152** | 5,373 | 95.9% |
| **identity.person.phone_e164** | 0 | **4,691** | 4,691 | 100.0% |
| **finance.banking.aba_routing** | 1,847 | **4,061** | 5,908 | 68.7% |
| **datetime.date.compact_ym** | 220 | **3,822** | 4,042 | 94.6% |
| **geography.transportation.iso6346** | 0 | **3,385** | 3,385 | 100.0% |
| **datetime.timestamp.slash_ymd_24h** | 81 | **3,320** | 3,401 | 97.6% |
| **datetime.timestamp.epoch_nanoseconds** | 58 | **3,274** | 3,332 | 98.3% |
| **technology.identifier.snowflake_id** | 36 | **2,807** | 2,843 | 98.7% |
| **datetime.time.hms_24h** | 411 | **2,659** | 3,070 | 86.6% |
| **finance.crypto.ethereum_address** | 79 | **2,501** | 2,580 | 96.9% |
| representation.numeric.percentage | 8,795 | 2,101 | 10,896 | 19.3% |
| datetime.date.mdy_slash | 6,787 | 1,999 | 8,786 | 22.8% |
| geography.location.country_code | 2,337 | 1,729 | 4,066 | 42.5% |
| datetime.epoch.unix_seconds | 76,693 | 1,716 | 78,409 | 2.2% |
| **datetime.timestamp.rfc_3339** | 67 | **1,356** | 1,423 | 95.3% |
| **finance.banking.iban** | 40 | **1,341** | 1,381 | 97.1% |
| **identity.government.ein** | 9 | **1,161** | 1,170 | 99.2% |
| **datetime.epoch.unix_microseconds** | 36 | **1,151** | 1,187 | 97.0% |

### Canary types (expect high refusal)

| YDF label | kept | refused | total | refusal % |
|---|---:|---:|---:|---:|
| geography.transportation.iso6346 | 0 | 3,385 | 3,385 | 100.0% |
| geography.coordinate.mgrs | 0 | 38 | 38 | 100.0% |
| geography.coordinate.plus_code | 0 | 4 | 4 | 100.0% |
| finance.payment.credit_card_number | 221 | 5,152 | 5,373 | 95.9% |
| identity.person.phone_e164 | 0 | 4,691 | 4,691 | 100.0% |

## v21

**5,978,485 predictions** — 5,794,984 kept, 183,501 refused (3.07% refusal rate).

### Top 25 by refusal count

| YDF label | kept | refused | total | refusal % |
|---|---:|---:|---:|---:|
| **datetime.component.year** | 17,992 | **40,532** | 58,524 | 69.3% |
| **representation.numeric.decimal_number_comma** | 1,256 | **19,548** | 20,804 | 94.0% |
| **identity.commerce.isbn** | 2,015 | **19,059** | 21,074 | 90.4% |
| representation.numeric.integer_number | 1,255,551 | 16,328 | 1,271,879 | 1.3% |
| **datetime.time.hm_24h** | 10,777 | **12,003** | 22,780 | 52.7% |
| representation.numeric.decimal_number | 1,787,302 | 7,413 | 1,794,715 | 0.4% |
| representation.identifier.alphanumeric_id | 47,051 | 5,619 | 52,670 | 10.7% |
| **finance.payment.credit_card_number** | 218 | **5,218** | 5,436 | 96.0% |
| **identity.person.phone_e164** | 0 | **4,691** | 4,691 | 100.0% |
| **finance.banking.aba_routing** | 1,819 | **3,965** | 5,784 | 68.6% |
| **datetime.date.compact_ym** | 222 | **3,815** | 4,037 | 94.5% |
| **geography.transportation.iso6346** | 0 | **3,387** | 3,387 | 100.0% |
| **datetime.timestamp.slash_ymd_24h** | 81 | **3,327** | 3,408 | 97.6% |
| **datetime.timestamp.epoch_nanoseconds** | 88 | **3,292** | 3,380 | 97.4% |
| **technology.identifier.snowflake_id** | 34 | **2,742** | 2,776 | 98.8% |
| **datetime.time.hms_24h** | 396 | **2,659** | 3,055 | 87.0% |
| **finance.crypto.ethereum_address** | 79 | **2,437** | 2,516 | 96.9% |
| representation.numeric.percentage | 8,795 | 2,103 | 10,898 | 19.3% |
| datetime.date.mdy_slash | 6,807 | 2,004 | 8,811 | 22.7% |
| geography.location.country_code | 2,340 | 1,728 | 4,068 | 42.5% |
| datetime.epoch.unix_seconds | 76,372 | 1,631 | 78,003 | 2.1% |
| **finance.banking.iban** | 40 | **1,350** | 1,390 | 97.1% |
| **datetime.timestamp.rfc_3339** | 69 | **1,269** | 1,338 | 94.8% |
| **identity.government.ein** | 9 | **1,160** | 1,169 | 99.2% |
| **datetime.epoch.unix_microseconds** | 36 | **1,150** | 1,186 | 97.0% |

### Canary types (expect high refusal)

| YDF label | kept | refused | total | refusal % |
|---|---:|---:|---:|---:|
| geography.transportation.iso6346 | 0 | 3,387 | 3,387 | 100.0% |
| geography.coordinate.mgrs | 0 | 38 | 38 | 100.0% |
| geography.coordinate.plus_code | 0 | 4 | 4 | 100.0% |
| finance.payment.credit_card_number | 218 | 5,218 | 5,436 | 96.0% |
| identity.person.phone_e164 | 0 | 4,691 | 4,691 | 100.0% |

## v22

**5,979,465 predictions** — 5,796,332 kept, 183,133 refused (3.06% refusal rate).

### Top 25 by refusal count

| YDF label | kept | refused | total | refusal % |
|---|---:|---:|---:|---:|
| **datetime.component.year** | 18,056 | **40,285** | 58,341 | 69.1% |
| **representation.numeric.decimal_number_comma** | 1,230 | **20,245** | 21,475 | 94.3% |
| **identity.commerce.isbn** | 2,024 | **18,288** | 20,312 | 90.0% |
| representation.numeric.integer_number | 1,257,457 | 16,974 | 1,274,431 | 1.3% |
| **datetime.time.hm_24h** | 10,781 | **11,997** | 22,778 | 52.7% |
| representation.numeric.decimal_number | 1,790,443 | 7,403 | 1,797,846 | 0.4% |
| representation.identifier.alphanumeric_id | 48,222 | 5,682 | 53,904 | 10.5% |
| **finance.payment.credit_card_number** | 186 | **4,985** | 5,171 | 96.4% |
| **identity.person.phone_e164** | 0 | **4,684** | 4,684 | 100.0% |
| **finance.banking.aba_routing** | 1,853 | **4,071** | 5,924 | 68.7% |
| **datetime.date.compact_ym** | 220 | **3,822** | 4,042 | 94.6% |
| **geography.transportation.iso6346** | 0 | **3,389** | 3,389 | 100.0% |
| **datetime.timestamp.slash_ymd_24h** | 81 | **3,321** | 3,402 | 97.6% |
| **datetime.timestamp.epoch_nanoseconds** | 21 | **3,078** | 3,099 | 99.3% |
| **technology.identifier.snowflake_id** | 32 | **2,724** | 2,756 | 98.8% |
| **datetime.time.hms_24h** | 383 | **2,659** | 3,042 | 87.4% |
| **finance.crypto.ethereum_address** | 79 | **2,520** | 2,599 | 97.0% |
| representation.numeric.percentage | 8,793 | 2,110 | 10,903 | 19.4% |
| datetime.date.mdy_slash | 6,741 | 1,903 | 8,644 | 22.0% |
| datetime.epoch.unix_seconds | 76,259 | 1,728 | 77,987 | 2.2% |
| geography.location.country_code | 2,329 | 1,715 | 4,044 | 42.4% |
| **datetime.timestamp.rfc_3339** | 67 | **1,390** | 1,457 | 95.4% |
| **finance.banking.iban** | 40 | **1,296** | 1,336 | 97.0% |
| **identity.government.ein** | 9 | **1,175** | 1,184 | 99.2% |
| **datetime.epoch.unix_microseconds** | 36 | **1,100** | 1,136 | 96.8% |

### Canary types (expect high refusal)

| YDF label | kept | refused | total | refusal % |
|---|---:|---:|---:|---:|
| geography.transportation.iso6346 | 0 | 3,389 | 3,389 | 100.0% |
| geography.coordinate.mgrs | 0 | 35 | 35 | 100.0% |
| geography.coordinate.plus_code | 0 | 4 | 4 | 100.0% |
| finance.payment.credit_card_number | 186 | 4,985 | 5,171 | 96.4% |
| identity.person.phone_e164 | 0 | 4,684 | 4,684 | 100.0% |

