# m2v->GBT witness sweep across gold buckets (sklearn HistGBT)

Sorted by abstaining-veto recall (max recall at precision>=0.90). value = Sharpen already owns it (witness redundant); SEMANTIC = the residual where the witness would be the unique lever.

| bucket                                      | kind | gold | train | AUC | R@P.90 | R@P.95 |
|---------------------------------------------|------|-----:|------:|----:|------:|------:|
| geography.location.country                  | SEMANTIC | 11 | 3244 | 0.995 | 0.818 | 0.818 |
| datetime.component.year                     | value | 41 | 1693 | 0.997 | 0.805 | 0.780 |
| geography.location.country_code             | value | 54 | 935 | 0.970 | 0.722 | 0.000 |
| identity.commerce.isbn                      | value | 18 | 1310 | 0.966 | 0.500 | 0.500 |
| technology.internet.url                     | value | 44 | 315 | 0.570 | 0.364 | 0.364 |
| representation.numeric.integer_number       | value | 193 | 4804 | 0.914 | 0.145 | 0.119 |
| geography.location.region                   | SEMANTIC | 15 | 4799 | 0.958 | 0.067 | 0.067 |
| geography.coordinate.latitude               | value | 39 | 286 | 0.446 | 0.026 | 0.026 |
| representation.text.word                    | SEMANTIC | 82 | 1583 | 0.886 | 0.012 | 0.012 |
| representation.numeric.decimal_number       | value | 94 | 2672 | 0.910 | 0.000 | 0.000 |
| representation.identifier.alphanumeric_id   | SEMANTIC | 62 | 2619 | 0.806 | 0.000 | 0.000 |
| datetime.date.iso                           | value | 52 | 318 | 0.706 | 0.000 | 0.000 |
| geography.coordinate.longitude              | value | 45 | 282 | 0.267 | 0.000 | 0.000 |
| representation.text.plain_text              | SEMANTIC | 41 | 8004 | 0.770 | 0.000 | 0.000 |
| geography.location.city                     | SEMANTIC | 24 | 3683 | 0.987 | 0.000 | 0.000 |
| datetime.epoch.unix_seconds                 | value | 15 | 317 | 0.306 | 0.000 | 0.000 |
| datetime.timestamp.sql_standard             | value | 14 | 302 | 0.127 | 0.000 | 0.000 |
| representation.boolean.terms                | value | 10 | 258 | 0.142 | 0.000 | 0.000 |
| representation.text.entity_name             | SEMANTIC | 8 | 12812 | 0.959 | 0.000 | 0.000 |
