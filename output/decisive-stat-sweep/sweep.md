# ac-04b decisive-stat sweep — gold slice precision

resolved 751/931 gold columns (180 unresolved)

gold support for asserted labels:
    67  representation.discrete.categorical
    52  representation.identifier.alphanumeric_id
     1  representation.identifier.increment
     0  representation.boolean.binary


## low_card->categorical
asserted label: representation.discrete.categorical  (gold support 67)
  knob  captured  correct  precision   top misfire labels
  0.01        76       25      0.329   integer_number:16, country_code:8, iso:6, region:4
  0.02       118       27      0.229   integer_number:35, country_code:12, iso:9, year:5
  0.05       184       35      0.190   integer_number:58, country_code:27, iso:15, year:7
   0.1       241       40      0.166   integer_number:84, country_code:30, iso:16, city:9
   0.2       286       51      0.178   integer_number:98, country_code:30, iso:17, decimal_number:12
   0.3       308       57      0.185   integer_number:103, country_code:30, iso:19, decimal_number:14
   0.5       370       60      0.162   integer_number:113, country_code:33, year:29, iso:26

## high_card+alnum->alphanumeric_id
asserted label: representation.identifier.alphanumeric_id  (gold support 52)
  knob  captured  correct  precision   top misfire labels
   0.5        79       46      0.582   url:19, plain_text:6, full_address:3, categorical:2
   0.7        74       43      0.581   url:19, plain_text:5, full_address:3, uuid:1
   0.9        63       39      0.619   url:17, full_address:3, plain_text:2, uuid:1
  0.95        62       38      0.613   url:17, full_address:3, plain_text:2, uuid:1
  0.99        60       38      0.633   url:17, full_address:3, uuid:1, csv:1
   1.0        59       37      0.627   url:17, full_address:3, uuid:1, csv:1

## increment_signature->increment
asserted label: representation.identifier.increment  (gold support 1)
  knob  captured  correct  precision   top misfire labels
   0.5        11        1      0.091   integer_number:7, year:3
   0.7         8        1      0.125   integer_number:6, year:1
   0.8         8        1      0.125   integer_number:6, year:1
   0.9         6        1      0.167   integer_number:4, year:1
  0.95         5        0      0.000   integer_number:4, year:1

## exact_binary->binary
asserted label: representation.boolean.binary  (gold support 0)
  knob  captured  correct  precision   top misfire labels
   0.0        26        0      0.000   integer_number:21, terms:4, decimal_number:1
  0.01        26        0      0.000   integer_number:21, terms:4, decimal_number:1
  0.05        26        0      0.000   integer_number:21, terms:4, decimal_number:1

## INSPECT exact_binary->binary @ knob 0.0 — what it fires on (gold-blind)
  gold=integer_number card=0.020 mn=0 mx=0 nd=1    col='modulation_default' vals=['0']
  gold=integer_number card=0.015 mn=0 mx=0 nd=1    col='modulation_default' vals=['0']
  gold=integer_number card=0.006 mn=0 mx=1 nd=2    col='SpeculativeGenerality_OneChildClass' vals=['0', '1']
  gold=integer_number card=0.250 mn=0 mx=1 nd=2    col='Platystomas sellatus' vals=['0', '1']
  gold=integer_number card=0.083 mn=0 mx=0 nd=1    col='perm_unlink' vals=['0']
  gold=integer_number card=0.133 mn=0 mx=1 nd=2    col='perm_unlink' vals=['0', '1']
  gold=integer_number card=0.222 mn=0 mx=1 nd=2    col='perm_unlink' vals=['0', '1']
  gold=integer_number card=0.200 mn=0 mx=1 nd=2    col='perm_unlink' vals=['0', '1']
  gold=integer_number card=0.001 mn=0 mx=0 nd=1    col='failure' vals=['0']
  gold=integer_number card=0.006 mn=0 mx=0 nd=1    col='tested' vals=['0']
  gold=integer_number card=0.077 mn=0 mx=0 nd=1    col='Comments' vals=['0']
  gold=integer_number card=0.002 mn=0 mx=0 nd=1    col='Smart 10: Spin_Retry_Count (Raw Value)' vals=['0']
  gold=integer_number card=0.030 mn=0 mx=0 nd=1    col='bps' vals=['0']
  gold=integer_number card=0.002 mn=0 mx=0 nd=1    col='Smart 198: Offline_Uncorrectable (Raw Value)' vals=['0']
  gold=integer_number card=0.020 mn=0 mx=0 nd=1    col='SET_TIME_ZONE' vals=['0']
  gold=terms          card=0.038 mn=0 mx=1 nd=2    col='beat_baseline-Covid19Sim-Simulator' vals=['False', 'True']
  gold=terms          card=0.043 mn=0 mx=1 nd=2    col='CanBeCloned' vals=['False', 'True']
  gold=integer_number card=0.071 mn=1 mx=1 nd=1    col='longTermDirection' vals=['1']
  gold=integer_number card=0.200 mn=0 mx=0 nd=1    col='3. State [5a7c1da369602a176d801427]' vals=['0']
  gold=integer_number card=0.200 mn=0 mx=0 nd=1    col='4. District [5a7c1da469602a176d801434]' vals=['0']
  gold=integer_number card=0.125 mn=1 mx=1 nd=1    col='rStatus' vals=['1']
  gold=integer_number card=0.200 mn=1 mx=1 nd=1    col='uri_Distinct' vals=['1']
  gold=integer_number card=0.077 mn=1 mx=1 nd=1    col='link_type' vals=['1']
  gold=terms          card=0.000 mn=0 mx=1 nd=2    col='scheduled_service' vals=['False', 'True']
  gold=terms          card=0.000 mn=0 mx=1 nd=2    col='arrest' vals=['False', 'True']
  gold=decimal_number card=0.015 mn=0 mx=0 nd=1    col='(year)' vals=['0.0']

## INSPECT increment_signature->increment @ knob 0.8 — what it fires on (gold-blind)
  gold=year           card=1.000 mn=2010 mx=2018 nd=9    col='Year' vals=['2010', '2015', '2012', '2011', '2017', '2018', '2013', '2014']
  gold=integer_number card=1.000 mn=9169 mx=9211 nd=43   col='ID' vals=['9206', '9204', '9201', '9200', '9199', '9195', '9191', '9186']
  gold=integer_number card=1.000 mn=1 mx=20000 nd=20000 col='GlobalRank' vals=['5', '9', '11', '14', '15', '16', '17', '22']
  gold=integer_number card=1.000 mn=117738 mx=117766 nd=29   col='ID' vals=['117764', '117760', '117755', '117754', '117753', '117752', '117749', '117747']
  gold=integer_number card=1.000 mn=2294476 mx=2294590 nd=102  col='ID' vals=['2294590', '2294580', '2294573', '2294568', '2294567', '2294561', '2294559', '2294557']
  gold=integer_number card=1.000 mn=155517 mx=155534 nd=16   col='ID' vals=['155534', '155532', '155531', '155528', '155526', '155522', '155517', '155527']
  gold=integer_number card=1.000 mn=122195 mx=122230 nd=36   col='ID' vals=['122230', '122227', '122226', '122225', '122221', '122217', '122212', '122211']
  gold=increment      card=1.000 mn=10568773 mx=10568882 nd=104  col='ID' vals=['10568877', '10568870', '10568869', '10568865', '10568864', '10568858', '10568856', '10568854']

[done]
