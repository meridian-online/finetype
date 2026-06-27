# ac-0 — Label-trust audit (clean-label retrain)

vocab sizes: city=38660 country=249 country_code=498 region_admin1=4322 region_improved=52862 person_tokens=49451 (given=24850 family=26548)

Membership keep rule: fraction of a column's values in the family vocab ≥ threshold → KEEP leaf, else DROP (noise).


## geography.location.city  (v3 rows: 3375, threshold 0.5)

**membership pass-rate: 2024/3375 = 60.0% would be KEPT**


Sample (verdict | frac | header | values):

- `KEEP` f=1.00 | **∅** | Los Angeles · Los Angeles · Los Angeles · Seoul · Seoul · Seoul · Barcelona
- `KEEP` f=0.90 | **∅** | Worcester · Chelmsford · Burlington · Tewksbury · Chelmsford · Lowell · Burlington · Andover · Hopkinton · Chelmsford · Westwood · Billerica  …(+8)
- `drop` f=0.00 | **∅** | Trego · Trego · Trego · Trego · Trego · Trego · Trego
- `KEEP` f=1.00 | **∅** | Minneapolis · South Burlington · Northampton · Oxford · Austin · Atlanta · Raleigh · Charleston · Asheville · Nashville
- `KEEP` f=0.55 | **∅** | San Miguel Duenas · San Antonio Aguas Calientes · Antigua Guatemala · Santa Maria de Jesus · Santa Catarina Barahona · Ciudad Vieja · Jocotenango · Pastores · Alotenango · Magdalena Milpas Altas · Santa Lucia Milpas Altas · Parramos  …(+8)
- `KEEP` f=0.60 | **∅** | Bratislava · Liptovsky Hradok · Humenne · Kezmarok · Hlohovec · Bytca · Prievidza · Presov · Nitra · Dubnica nad Vahom · Kralovsky Chlmec · Hurbanovo  …(+3)
- `KEEP` f=0.87 | **∅** | Washington · New York · Philadelphia · Allston · Montreal · Pontiac · Chicago · Minneapolis · Vancouver · Seattle · San Francisco · Los Angeles  …(+3)
- `drop` f=0.29 | **∅** | Alexandria · Arlington · Fairfax County · Fairfax City · Falls Church · Loudoun County · Prince William County
- `drop` f=0.00 | **∅** | Palmico Sound, NC · Wilmington, NC · New Bern, NC · Cape Hatteras, NC · Cedar Island, NC
- `drop` f=0.00 | **∅** | Atlanta, GA · ? · Daytona Beach, FL · Tallahassee, FL · Atlanta, GA · Macon, GA · Orangeburg, SC · Nashville, TN · Atlanta, GA · Atlanta, GA
- `KEEP` f=1.00 | **∅** | Tampa · Tampa · Tampa · Tampa · Tampa · Tampa · Tampa · Tampa · Tampa · Tampa
- `drop` f=0.00 | **∅** | Hamilton, NY · Hamilton, NY · Hamilton, NY · Hamilton, NY · Hamilton, NY · Hamilton, NY
- `KEEP` f=0.93 | **∅** | London · Bristol · Dublin · Leeds · Nashville · New York · San Francisco · Denver · Miami · Washington · Philadelphia · Austin  …(+3)
- `KEEP` f=0.70 | **∅** | Aberdeen · Aberdeen · Aberdeen · Aberdeen · Aberdeen · Abergerveny · Aberysthwyth · Aberysthwyth · Angel, Islington, London · Ashby De La Zouch · Bangor · Barnet, London  …(+8)
- `KEEP` f=0.50 | **∅** | Lake Placid · Lake Placid · Lake Placid · Lake Placid · Lake Placid · Sarajevo · Sarajevo · Sarajevo · Sarajevo · Sarajevo
- `drop` f=0.00 | **∅** | Totals: · ABSAROKA MOUNTAINS (ZONE) · WIND RIVER MOUNTAINS EAST (ZON... · HYATTVILLE · BUFFALO ARPT · MAYOWORTH · TEN SLEEP ARPT · Totals:
- `drop` f=0.00 | **∅** | Kingston, RI · Orono, ME · Orono, ME · Orono, ME · Orono, ME · Brunswick, ME · Durham, NH · Orono, ME
- `drop` f=0.40 | **∅** | San Diego · Las Vegas · Hampton Beach · Westhampton Beach · Hampton Beach · Farmington · Glen Allen · Webster · New York · Thousand Oaks
- `drop` f=0.00 | **∅** | DETOUR · HESSEL · NEWBERRY · ECKERMAN · DETOUR VILLAGE · DAFTER
- `drop` f=0.38 | **∅** | München · München · München · München · München · München · München · München · Montréal · Montréal · Montréal · Montréal  …(+1)
- `KEEP` f=0.60 | **∅** | E. Amherst · Delmar · Wantagh · Mahopac · Newmarket · Massapequa · Wellesley Hills · Manlius · Minnetonka · Webster · Lincoln · Chatham  …(+8)
- `drop` f=0.00 | **∅** | Rockwall, Texas · Abernathy, Texas · Abilene, Texas · Abilene, Texas · San Antonio, Texas · Alamo, Texas · San Antonio, Texas · Albany, Texas · Alice, Texas · Alice, Texas · Alpine, Texas · Alpine, Texas  …(+8)

_CLEAN generator sample for geography.location.city (12 shown):_
  - **city** | Plufur · Vitré · Lannepax · Mailly-Maillet · Plouër-sur-Rance · Mairy-sur-Marne · Ravenel · NULL · Baugé-en-Anjou · Luzenac · Saint-Hippolyte · Saint-Rémy  …(+78)
  - **city** | Haverhill · West Wood · Irving · Richland · Wilmot · Livingston · Rochester · Clay City · Salamanca · Winfield · York Springs · London  …(+83)
  - **cidade** | Macapá · São João da Canabrava · Almadina · Nhandeara · Mire de Tibães · Guarda · Laurentino · Iraí · Marechal Cândido Rondon · Chiapetta · Governador Lindenberg · Goianorte  …(+88)
  - **comune** | Configni · Cirignano · Scortichino · Saint Marcel · Mas · Orsogna · Sibari · Lenno · Lizzano in Belvedere · Noragugume · Badetto · Lauria  …(+88)
  - **stadt** | Ferlach · Soltau · Simbach · Vahlde · Breitenbach · Ilvesheim · Frankenstein · Korbach · Tellig · Eisfeld · Altes Hochschulviertel · Winsen  …(+84)
  - **city** | Farrera (Spain) · Estepona (Spain) · Alcalá del Río (Spain) · Valdemanco del Esteras (Spain) · Los Angeles (Spain) · Los Rosales (Spain) · Galende (Spain) · Encinillas (Spain) · Santa Úrsula (Spain) · Tomelloso (Spain) · Humilladero (Spain) · Velilla del Río Carrión (Spain)  …(+88)
  - **localité** | Lempdes · Germigny · Bressolles · Lahonce · Herbeys · Marsaneix · Saint-Roch · Guéreins · Torcieu · Vecoux · Saint-Sauveur-en-Rue · Pocé-sur-Cisse  …(+88)
  - **plaats** | Hellebecq · Veenendaal · Leuven · Eigenbrakel · Chevron · Warnant-Dreye · Hamipré · Nieuwoord · Houtvenne · NULL · Büllingen · Bois-et-Borsu  …(+83)
  - **city** | Rouans · Lignerolles · Queaux · Thiéblemont-Farémont · Nieul · Chanac · Bonneville · Tabanac · Boissy-Mauvoisin · Marlens · Clénay · Saint-Denis-lès-Rebais  …(+88)
  - **도시** | 양산 · Hoenam · 서울 · Boseong · 경산시 · 칠보 · 승주 · Yeongdong · Chinch'ŏn · 구룡포 · 공주시 · 한천  …(+88)


## geography.location.country  (v3 rows: 2943, threshold 0.5)

**membership pass-rate: 2460/2943 = 83.6% would be KEPT**


Sample (verdict | frac | header | values):

- `KEEP` f=0.80 | **∅** | BERMUDA · U.S. · GERMANY · FRANCE · SPAIN
- `drop` f=0.00 | **∅** | Exitmusic · Exitmusic · Exitmusic · Exitmusic · Exitmusic
- `KEEP` f=0.90 | **∅** | Argentina · Armenia · Australia · Belgium · Botswana · Brazil · Canada · Chile · China (mainland) · Denmark
- `drop` f=0.10 | **∅** | United States · Canada · United States · United States · United States · United States · United States · United States · United States · United States
- `drop` f=0.00 | **∅** | Argentina: · Spain: · Mexico: · Venezuela: · Chile: · Guatemala: · Brazil: · United States:
- `KEEP` f=0.50 | **∅** | United States · United States · Guam · Canada · United States · United States · Canada · United States · Canada · Canada
- `KEEP` f=1.00 | **∅** | Afghanistan · Albania · Germany · Andorra · Saudi Arabia · Armenia · Australia · Austria · Azerbaijan · Bahrain
- `KEEP` f=0.88 | **∅** | Brazil · United States · Canada · Paraguay · France · Ukraine · Slovakia · Hong Kong
- `KEEP` f=0.71 | **∅** | Germany · Bangladesh · Neth. · Hong Kong · Switz. · Australia · Iceland
- `KEEP` f=0.75 | **∅** | Brazil · N/A · United States · Portugal · Mozambique · Japan · Argentina · Mexico
- `KEEP` f=0.57 | **∅** | Brazil · Portugal · United States · N/A · United Kingdom · Japan · France
- `KEEP` f=1.00 | **∅** | Argentina · Argentina · Argentina · Australia · Australia · Australia · Brazil · Brazil · Brazil · Canada
- `KEEP` f=1.00 | **∅** | Albania · Andorra · Armenia · Austria · Belarus · Belgium · Bosnia and Herzegovina · Bulgaria · Croatia · Cyprus
- `KEEP` f=0.75 | **∅** | Hungary · United Kingdom · Romania · Serbia · Germany · Slovakia · (not set) · Austria · Netherlands · United States · Portugal · France  …(+8)
- `KEEP` f=0.89 | **∅** | Bangladesh · Cambodia · DRC · Ethiopia · Ghana · India · Kenya · Malawi · Mozambique
- `KEEP` f=0.85 | **∅** | China · Romania · Mexico · Poland · China · Not stated · China · Philippines · Morocco · India · Bolivia · Romania  …(+8)
- `drop` f=0.33 | **∅** | Brazil · N/A · United Kingdom · United States · Venezuela · Paraguay
- `drop` f=0.00 | **∅** | Tsuru Capital LLC · Tsuru Capital SG Pte Ltd · bCODE Pty Ltd · IIJ · Google
- `drop` f=0.33 | **∅** | United States · Brazil · China · N/A · Netherlands · Europe
- `KEEP` f=0.90 | **∅** | Azerbaijan · Armenia · Argentina · Australia · Belarus · Canada · Denmark · Estonia · Czech Republic · Georgia
- `drop` f=0.00 | **∅** | United States · United States · Ivory Coast · United States · United States
- `KEEP` f=0.70 | **∅** | Bulgaria · Croatia · Czech Republic · Czech Republic · Germany · Hungary · Poland · Poland · Poland · Russia

_CLEAN generator sample for geography.location.country (12 shown):_
  - **country_name** | Anguilla · Uzbekistan · Syria · Cameroon · united arab emirates · gibraltar · Northern Mariana Islands · CURACAO · Dominican Republic · China · South Africa · Grenada  …(+88)
  - **country** | Lebanon · Iraq · Sweden · Saint Barthelemy · Suriname · Malta · Liechtenstein · Sierra Leone · Colombia · Jersey · Indonesia · United Arab Emirates  …(+88)
  - **국가** | Jersey · Aruba · Morocco · Ecuador · United Kingdom · Finland · Bouvet Island · Malta · Brazil · Cabo Verde · Greenland · Guyana  …(+88)
  - **country** | Nepal · Republic of the Congo · Philippines · Sri Lanka · Uganda · Laos · Equatorial Guinea · Syria · Dominica · Kenya · Marshall Islands · Christmas Island  …(+88)
  - **country** | Montenegro · North Macedonia · Vietnam · United Arab Emirates · Tuvalu · Vatican · Sao Tome and Principe · Solomon Islands · Azerbaijan · Mali · Jamaica · Saudi Arabia  …(+88)
  - **país** | Heard Island and McDonald Islands · Vanuatu · Burkina Faso · Pakistan · Equatorial Guinea · Czechia · Mexico · Republic of the Congo · Greece · Slovakia · Central African Republic · Cuba  …(+88)
  - **staat** | Micronesia · Cook Islands · Luxembourg · Western Sahara · Fiji · Nauru · Mayotte · Chad · Faroe Islands · Wallis and Futuna · Iraq · Palestinian Territory  …(+88)
  - **country** | Niue · Peru · Uzbekistan · Egypt · Norway · Dominica · Andorra · Solomon Islands · Monaco · Yemen · Haiti · Switzerland  …(+88)
  - **pays** | Norway · Denmark · Comoros · Tokelau · Wallis and Futuna · Jamaica · Liberia · Estonia · Guatemala · Barbados · Armenia · Guam  …(+88)
  - **country** | somalia · india · Liechtenstein · greenland · Georgia · Nepal · iran · mongolia · Jersey · Togo · BENIN · Saudi Arabia  …(+88)


## geography.location.country_code  (v3 rows: 619, threshold 0.5)

**membership pass-rate: 483/619 = 78.0% would be KEPT**


Sample (verdict | frac | header | values):

- `KEEP` f=1.00 | **∅** | DE · US · AU · DE · US · US
- `KEEP` f=0.57 | **∅** | POL · ROU · NED · SLO · ITA · SUI · MKD
- `KEEP` f=1.00 | **∅** | USA · GBR · USA · FRA · PAK · LBN · GBR · ZWE · USA · ZWE · CRI · PRT  …(+8)
- `KEEP` f=0.75 | **∅** | RUS · CZE · UKR · SWE · GER · ITA · FRA · NED
- `drop` f=0.17 | **∅** | US · UK · China · India · Germany · France
- `KEEP` f=1.00 | **∅** | US · US · US · US · US · US · US · US · US · US · US · US  …(+8)
- `KEEP` f=0.70 | **∅** | IN · IL · KY · PA · PA · MA · MA · VT · CT · NY
- `KEEP` f=0.70 | **∅** | CHN · CHN · KEN · KEN · USA · POR · POR · USA · GER · POR · BRA · ITA  …(+8)
- `KEEP` f=0.62 | **∅** | Ind · Ind · Mas · Aus · Aus · Mas · Eng · Can
- `drop` f=0.20 | **∅** | IA · IL · KS · MN · NY · OH · OR · WI · NJ · WA
- `KEEP` f=0.95 | **∅** | LT · NZ · SA · CO · ZA · EG · GT · CY · JE · PK · OM · SI  …(+8)
- `KEEP` f=0.60 | **∅** | OH · IL · MN · MO · CO · CA · CA · NV · TX · TX
- `KEEP` f=0.89 | **∅** | FRA · NED · BLR · EST · BEL · LTU · RUS · BLR · SRB
- `KEEP` f=0.60 | **∅** | GB · Ger · Ger · Fin · Fin · Brz · Aus · Rus · Ger · Fra
- `KEEP` f=1.00 | **∅** | CO · CO · CO · CO · CO
- `KEEP` f=0.70 | **∅** | CA · NV · CA · MI · CA · CA · NV · CA · CA · CA
- `KEEP` f=0.70 | **∅** | POR · ITA · SWE · SUI · GER · CZE · AUT · SUI · FRA · RUS · HUN · UKR  …(+8)
- `KEEP` f=1.00 | **∅** | FRA · BLR · UKR · EST · GBR · FRA · FRA · EST · RUS
- `KEEP` f=0.65 | **∅** | DC · CT · MA · PA · VA · NC · GA · FL · FL · FL · AL · AL  …(+8)
- `drop` f=0.00 | **∅** | KS · KS · KS · KS · KS · KS · KS · KS · KS · KS · KS · KS  …(+2)
- `drop` f=0.40 | **∅** | RSA · NAM · RSA · ZIM · MRI · KEN · EGY · UGA · ZIM · ZIM
- `KEEP` f=1.00 | **∅** | CA · CA · CA · CA · CA · MD · MA

_CLEAN generator sample for geography.location.country_code (12 shown):_
  - **iso** | BDI · SOM · GNB · SDN · SYR · BGR · ASM · CAN · CCK · GTM · KWT · GIN  …(+88)
  - **code_pays** | cri · gnq · UMI · slb · PCN · GRD · xkx · Rwa · iot · DNK · Cyp · ARG  …(+88)
  - **country_code** | DZ · sr · Cg · AU · TW · Pa · Bt · am · Sg · IO · gw · AF  …(+88)
  - **country_code** | CM · GA · MF · IT · JP · AN · EC · FR · BA · UG · HN · CY  …(+88)
  - **国コード** | grc · dnk · ALB · vgb · ANT · BES · Mdg · SLE · sdn · VIR · BHS · MWI  …(+88)
  - **country_code** | MOZ · bgr · Prt · ATF · alb · guf · Nor · Slv · blr · Rwa · tun · SHN  …(+88)
  - **iso** | VE · FI · MN · MR · ZA · TO · AR · KI · PS · CO · ER · TM  …(+88)
  - **código_país** | guf · ltu · SSD · Fji · Gha · VGB · Chl · Bhs · KWT · GNB · Kgz · CMR  …(+88)
  - **código_país** | VN · BD · SS · UG · SA · XK · IT · KR · BT · MD · DM · PF  …(+88)
  - **country_code** | MHL · UZB · TUR · ZAF · SRB · CXR · MUS · MEX · ISR · ROU · LBR · ETH  …(+88)


## geography.location.region  (v3 rows: 4477, threshold 0.4)

**membership pass-rate: 3615/4477 = 80.7% would be KEPT**

**region collapse check — admin1-only pass-rate: 956/4477 = 21.4%** (improved: 80.7%; the gap is the collapse the fix recovers)


Sample (verdict | frac | header | values):

- `KEEP` f=1.00 | **∅** | Rutherford · Davidson · Gibson · Montgomery · Henderson · Knox · Madison · Gibson · Hamilton · Shelby · Shelby · Crockett  …(+8)
- `KEEP` f=1.00 | **∅** | Lazio · Lazio · Lazio · Lazio · Lazio · Lazio · Lazio · Lazio · Lazio · Lazio · Lazio · Lazio  …(+3)
- `KEEP` f=1.00 | **∅** | TX · TX · OK · MS · NJ · NJ · VA · NC · SC · NY · LA · KY  …(+8)
- `KEEP` f=1.00 | **∅** | Indiana · Michigan · Kentucky · Ohio · Oklahoma
- `KEEP` f=1.00 | **∅** | NY · MA · DC · NC · MN · MA · OH · KS · MS · NC · WV · AL  …(+8)
- `KEEP` f=1.00 | **∅** | Alabama · Alaska · Arizona · Arkansas · California · Colorado · Connecticut · Delaware · District of Columbia · Florida · Georgia · Hawaii  …(+8)
- `KEEP` f=0.88 | **∅** | BC · ID · MO · MI · NY · MA · CA · CA
- `KEEP` f=1.00 | **∅** | Stewart · Monroe · Greene · Cocke · Blount · Obion · Knox · Wilson · Washington · McMinn · Warren · Gibson  …(+8)
- `KEEP` f=0.67 | **∅** | Glen Eira · Southern Metro · Victoria · Glen Eira · Southern Metro · Victoria
- `drop` f=0.00 | **∅** | cm­b10.tfm · cmb­sy10.tfm · cm­bx10.tfm · cm­bx12.tfm · cm­bx5.tfm · cm­bx6.tfm · cm­bx7.tfm · cm­bx8.tfm · cm­bx9.tfm · cm­bxs­l10.tfm · cm­bx­ti10.tfm · cm­c­sc10.tfm  …(+8)
- `KEEP` f=1.00 | **∅** | Grainger · Hamilton · Moore · Shelby · Hamilton · Gibson · Washington · McMinn · Washington · Obion
- `KEEP` f=1.00 | **∅** | FL · GA · NJ · MA · RI · PA · MD · KY · OH · OH · MI · MI  …(+8)
- `drop` f=0.00 | **∅** | Youth Only · Youth Only · Youth Only · Youth Only · Regional C · Regional C · Youth Only · Youth Only · Youth Only · Youth Only · Youth Only · Youth Only  …(+8)
- `KEEP` f=1.00 | **∅** | Washington · Washington · Washington · Washington · Washington · Washington · Washington
- `KEEP` f=0.55 | **∅** | Johor · Sabah · Pahang · Colorado · Colorado · Colorado · Tanzania · Sumatra · India · Taiwan · Taiwan · Switzerland  …(+8)
- `KEEP` f=1.00 | **∅** | Alberta · Alberta · Alberta · Alberta · Alberta · Alberta · Alberta · Alberta · Alberta · Alberta · Alberta · Alberta  …(+8)
- `KEEP` f=1.00 | **∅** | Hillsborough · Pasco · Pinellas · Polk · Sarasota · Hernando · Manatee · Florida
- `KEEP` f=1.00 | **∅** | VIRGINIA · ACCOMACK · ALBEMARLE · ALLEGHANY · AMELIA · AMHERST · APPOMATTOX · ARLINGTON · AUGUSTA · BATH · BEDFORD · BLAND  …(+8)
- `KEEP` f=1.00 | **∅** | Georgia · Georgia · Georgia · Georgia · Georgia · Georgia · Georgia · Georgia · Georgia · Georgia · Georgia · Georgia  …(+4)
- `KEEP` f=1.00 | **∅** | Clay · Blount · Hamblen · Knox · Fayette · Polk · Sevier · Blount · Lincoln · Lincoln
- `drop` f=0.00 | **∅** | ARIZ · ARIZ · ARIZ · ARIZ · ARIZ
- `KEEP` f=1.00 | **∅** | NY · NY · PA · NY · OH · MI · MO · TN · FL · ME · NJ · FL  …(+8)

_CLEAN generator sample for geography.location.region (12 shown):_
  - **département** | Hauts-de-France · New Aquitaine · Grand Est · Île-de-France · Île-de-France · Île-de-France · Occitanie · Occitanie · Occitanie · Brittany · Grand Est · Provence-Alpes-Côte d'Azur  …(+88)
  - **bundesland** | Lower Saxony · Zurich · Basel-City · Glarus · Thuringia · Schwyz · Thurgau · Hamburg · State of Berlin · Zurich · Vorarlberg · Uri  …(+88)
  - **region** | Styria · Bern · Valais · North Rhine-Westphalia · Zug · Vaud · Lower Saxony · State of Salzburg · Carinthia · Uri · Bremen · Bavaria  …(+86)
  - **область** | Mangystau · ABAI REGION · KOSTANAY · Stavropol Kray · Smolensk Oblast · West Kazakhstan · KABARDINO-BALKARIYA REPUBLIC · Sakha · Astana · kaliningrad oblast · ZAKARPATTIA · Khmelnytskyi  …(+88)
  - **state** | Illinois · District of Columbia · New Hampshire · South Carolina · California · Texas · Pennsylvania · Alaska · North Dakota · Tennessee · Wisconsin · Indiana  …(+88)
  - **region** | Pomerania · Silesia · Subcarpathia · Lower Silesia · Pomerania · Mazovia · Świętokrzyskie · Subcarpathia · Greater Poland · Opole Voivodeship · Lesser Poland · Świętokrzyskie  …(+80)
  - **land** | Vorarlberg · Hamburg · State of Vienna · Baden-Wurttemberg · Lucerne · Basel-City · Mecklenburg-Vorpommern · Brandenburg · Bremen · Burgenland · Solothurn · Glarus  …(+88)
  - **regio** | Limburg · Zeeland · Zeeland · Wallonia · Flanders · Wallonia · North Holland · Zeeland · Flevoland · Limburg · Limburg · Flanders  …(+84)
  - **region** | Gwangju · North Chungcheong · Daejeon · North Chungcheong · Gwangju · Gwangju · Chungcheongnam-do · Sejong-si · Ulsan · Daejeon · Jeollabuk-do · North Chungcheong  …(+88)
  - **kanton** | Val · Vorarl · Schaffh · Lower S · Sax · Z · Saint G · Val · Bava · Z · Zur · Gla  …(+88)


## geography.location.continent  (v3 rows: 283, threshold 0.5)

**membership pass-rate: 246/283 = 86.9% would be KEPT**


Sample (verdict | frac | header | values):

- `KEEP` f=1.00 | **∅** | North America · North America · North America · Europe · North America · North America · North America · North America · North America · North America
- `drop` f=0.20 | **∅** | Western Europe · Western Europe · Western Europe · Western Europe · Western Europe · Western Europe · Western Europe · Western Europe · Western Europe · Western Europe · Western Europe · Western Europe  …(+8)
- `KEEP` f=1.00 | **∅** | AS · OC · OC · SA · EU · AS · EU · EU · AS · EU · EU · EU  …(+7)
- `KEEP` f=1.00 | **∅** | EU · EU · AS · AS · OC · NA
- `KEEP` f=0.80 | **∅** | Europe · North America · North America · Europe · Japan
- `KEEP` f=1.00 | **∅** | SA · OC · EU · EU · OC · AS · AS · EU
- `KEEP` f=1.00 | **∅** | Europe · Europe · Europe · South America · Europe · South America
- `KEEP` f=0.86 | **∅** | Asia · South America · North America · Africa · Europe · Antarctica · Australia - New Guinea
- `KEEP` f=0.67 | **∅** | Japan · North America · Europe · North America · Japan · Europe · Europe · North America · Japan · North America · Japan · Europe
- `KEEP` f=1.00 | **∅** | Africa · Africa · Africa · Africa · Africa · Africa · Africa · Africa · Africa · Africa
- `KEEP` f=0.65 | **∅** | Japan · Japan · Japan · Japan · Japan · Japan · Europe · Japan · North America · Europe · North America · North America  …(+8)
- `KEEP` f=1.00 | **∅** | AS · EU · AS · EU · EU · EU · NA · EU
- `KEEP` f=0.90 | **∅** | Aisa · Europe · Europe · Europe · Europe · South America · Europe · Australia · Europe · Europe
- `KEEP` f=1.00 | **∅** | EU · EU · EU · EU · EU · AS · EU
- `KEEP` f=0.50 | **∅** | Asia & Oceania · North America · Europe · Eurasia · Central & South America · Africa
- `drop` f=0.29 | **∅** | Europe · CIS** · The Americas · World · Arab States · Asia & Pacific · Africa
- `KEEP` f=0.67 | **∅** | Africa · Americas · Asia · Europe · Oceania · World
- `drop` f=0.40 | **∅** | E · AF · SA · E · E · E · A · E · SA · SA
- `KEEP` f=1.00 | **∅** | Europe · Africa · Europe · Europe · Europe · Europe · Europe · Europe · Europe · Europe · Europe · Europe  …(+7)
- `KEEP` f=0.57 | **∅** | africa · asia · central_america · europe · north_america · oceania · south_america
- `KEEP` f=0.50 | **∅** | Asia · Europa · Nord-centro America · Asia · Asia · Sud America · Europa · Europa · Asia · Europa · Asia · Asia  …(+8)
- `KEEP` f=0.70 | **∅** | Africa · Oceania · Africa · Africa · Africa · Europe · Africa · Africa · Africa · Africa · Asia · Asia  …(+8)

_CLEAN generator sample for geography.location.continent (12 shown):_
  - **continent** | As · Eu · AF · na · AF · af · Af · EU · As · af · NA · AS  …(+88)
  - **continente** | AF (Africa) · NA (North America) · OC (Oceania) · AF (Africa) · AF (Africa) · AS (Asia) · NA (North America) · NA (North America) · EU (Europe) · EU (Europe) · AF (Africa) · EU (Europe)  …(+88)
  - **continent** | EU · EU · EU · AF · AS · EU · AF · AS · SA · AS · SA · AF  …(+83)
  - **continent_code** | AF (Africa) · AN (Antarctica) · AF (Africa) · AF (Africa) · SA (South America) · AF (Africa) · NA (North America) · EU (Europe) · AF (Africa) · AF (Africa) · NA (North America) · SA (South America)  …(+88)
  - **kontinent** | AS (Asia) · AF (Africa) · AF (Africa) · AS (Asia) · AS (Asia) · AF (Africa) · AF (Africa) · OC (Oceania) · AS (Asia) · NA (North America) · NA (North America) · EU (Europe)  …(+88)
  - **continente** | NA · AS · AF · NA · AS · AF · NA · AS · AS · AF · EU · OC  …(+88)
  - **continent** | EU · NA · EU · AF · NA · NA · AF · AF · OC · SA · NA · AF  …(+83)
  - **kontinent** | AF · AS · AS · AF · NA · AS · NA · EU · NA · EU · EU · EU  …(+88)
  - **continent** | OC · AF · EU · AS · SA · AF · NULL · OC · AS · AF · AS · OC  …(+83)
  - **大洲** | AF (Africa) · OC (Oceania) · AF (Africa) · EU (Europe) · EU (Europe) · SA (South America) · OC (Oceania) · OC (Oceania) · EU (Europe) · NA (North America) · AS (Asia) · AN (Antarctica)  …(+88)


## identity.person.full_name  (v3 rows: 5262, threshold 0.5)

**membership pass-rate: 3321/5262 = 63.1% would be KEPT**


Sample (verdict | frac | header | values):

- `KEEP` f=0.67 | **∅** | D E Mullins · D E Mullins · D E Mullins · D E Mullins · D E Mullins · Mr J T Carroll · Mr J T Carroll · D E Mullins · Mr J T Carroll
- `drop` f=0.14 | **∅** | P T Enright · Mr P W Mullins · N P Madden · T J Doyle · B J Cooper · P Townend · M J Ferris
- `KEEP` f=0.50 | **∅** | White, Saxon William · Ostwald, Michael J. · Byles, Julie · Sibbritt, David · Morgan, Philip J. · Borwein, Jonathan M. · Loxton, Deborah · Dobson, Annette · Gray, Mel · Collins, Clare E. · Crump, S. J. · Williams, Anthony  …(+8)
- `drop` f=0.30 | **∅** | Constantino Roman · Julio Felix · Timothy Thornton · Florent Geroux · Israel Ocampo · Nicholas Meza · Alex Canchari · Rosemary Jr Homeister · Christopher Emigh · Alejandro Contreras
- `KEEP` f=1.00 | **∅** | Brandon May · Bernard Fayd'Herbe · Robert Khathi · Robert Khathi · Robert Khathi · Fareed Anthony · Christopher Puller · Devin Ashby · Devin Ashby · K Zechner
- `KEEP` f=0.94 | **∅** | Callan Murray · A Forbes · Warren Kennedy · Warren Kennedy · Warren Kennedy · Derryl Daniels · Donovan Dillon · Donovan Dillon · S Randolph · A Forbes · A Forbes · J Samuel  …(+5)
- `KEEP` f=0.60 | **∅** | Adam Haggerton · Nancy Haggerton · John H. Haggerton · Charles M. Haggerton · Gibbs E. Haggerton
- `KEEP` f=0.70 | **∅** | Manuel Sanguily · Bob Best · John Kortheuer · Robert C MacDonald · Ashley O Jones · Norman E Stupfel · Charles Baldwin · Milton Marks · Charles Urstadt · Ted Haartz
- `KEEP` f=0.60 | **∅** | J Ortiz · M Franco (5) · C H Velasquez · J Alvarado · A Jr (7) · J Castellano · R Maragh · A Lezcano · P Fragoso · G Rodriguez
- `KEEP` f=0.55 | **∅** | Capell, Cortez · Kantor, Dan · Featherston, Chandler · Sonnenberg, Chris · Ehrilch, Scott · Pierce, DJ · Eskanos, RJ · Droge, Chris · Rodby, Kirk · Skuro, Sean · Player, Sub
- `KEEP` f=0.82 | **∅** | Tyler Pizarro · Sheldon Russell · Emma Jayne Wilson · James McAleney · Patrick Husbands · A Solis · Jesse Campbell · John Velazquez · Luis Contreras · Justin Stein · David Moran
- `KEEP` f=0.50 | **∅** | P Hanagan · P Hanagan · P Hanagan · P Mulrennan · F Norton · J Fanning · J Fanning · Dane O'Neill
- `KEEP` f=0.60 | **∅** | Levinson, David · Chaffee, Marc · Levinson, Dave · Drexelius Sr, Daniel · Mallare, Marc
- `KEEP` f=0.56 | **∅** | R Montanez · M Remedio · R Chiappe · B Pedroza · J Arce · J Acosta · R Moya · A Suarez (5) · J Torres
- `KEEP` f=0.50 | **∅** | Dylan Sneed · Dave Matthews and Tim Reynolds · Keith Alan Mitchell · Casey Neill · Dave Van Ronk · Dave Carter and Tracy Grammer · Mollie O'Brien and Rich Moore · Christine Parker · John Roy Zat · The Macrae Sisters
- `KEEP` f=0.60 | **∅** | Brandin Daniel · Yunior Tabares · Braulio Pardo · Jason Hamm · Bobby Munoz · Kyle Hobbs · Jonathan Duncan · J.c. Suarez · T.j. Alonzo · Dale Slimick
- `KEEP` f=0.80 | **∅** | WAYNE R · DAVID E · RAFAEL E · Jean-Pierre · WILLIAM H
- `KEEP` f=0.69 | **∅** | Cheryl Wheeler · Ken Gaines · John Gorka · John McCutcheon · Greg Brown · Bill Morrissey · Jon Ims · Tom Prasada-Rao · John Stewart · Stark Raving Chandler · Tom Paxton · Crow Johnson  …(+4)
- `KEEP` f=0.60 | **∅** | Andy Warhol · Tom Wesselmann · Roy Lichtenstein · Robert Rauschenberg · Robert Indiana
- `drop` f=0.30 | **∅** | Richard T Abrahams · Hugh Wilder · David Quiggin · Keefe L Lodwig · Daniel J Rogacki · James R DeLacy · Andrew M McPherson · Peter Andersen · Jan Soderstrom · Rick Meyerhoff
- `drop` f=0.30 | **∅** | Soulja Boy Tell'em · Rihanna · Alicia Keys · Sean Kingston · Chris Brown · Maroon5 · Plain White T's · Fergie · Akon · Mims
- `KEEP` f=1.00 | **∅** | Jackson 5 · Jackson 5 · Michael Jackson ft. Will I Am · Michael Jackson · Michael Jackson

_CLEAN generator sample for identity.person.full_name (12 shown):_
  - **user** | F. Gregory · B. Nakagawa · B. Vukašin · O. Bontje · L. De Thou · T. Danvers · L. Nabulsi · R. Dimsdale · E. Ejercito · K. Blūšius · A. Dayes · L. Gilkes  …(+26)
  - **Plaintiff** | Adegbindin Biernaski · Lipp Čepič · Zepherina Anđelović · Jun Mihelič · Doetval Burnet · Andet Hendryx · Mareille Huska · Auda Bloomquist · Abu'l-Fadl Freilich · Ready Baranka · Zixiang Gorse · Gabisiu Bruton  …(+48)
  - **Defendant** | Yìngxiáng Dodgson · Karèya Choge · Charleta Ax · Yvonnecris Stonys · Flera Matthysse · Twiggy Čepėnas · Mardi Lira · Çano Uzdila · Shukria Dağ · Hutham Engels · Isamedin Štebi · Jong-gwan Higgins  …(+29)
  - **person** | Pradyut Čeplak III · Arijdina Carreras Sr. · Stradling Tsutsumi Sr. · Attwater Bibb II · Powel Gounter II · Un-bong Bozhenko II · Ilsetraut Moțoc III · Xiang Trojan Esq. · MacGeoch Aspland II · Colles Daugintis III · Slava Eichhorn III · Hysein Petelin IV  …(+35)
  - **Author** | RAIES HAUSER · GENTJAN EASLEY · KONEL HERTLING · DIEMO KRZYŻEWSKI · GOODACRE VANDERVIS · TIMMON POULETT · SHAHED MANABE · SEF WHEELHOUSE · COLLENE IDZINGA · MYZEMIL BOZGA · HOME JAMERSON · ILDIJE HEIKKA  …(+52)
  - **Name** | Ahsee Manalo · Grainger Prijatelj · Mikan Dobrynin · Bencie Currier · Lomas Dědič · Kricheldorf Grillo · Edvânio Bradbery · Gaeng Anger · Burrage Werder · Hemphill Bamford · Jiu Cardon · Dansa Dethick  …(+43)
  - **reviewer** | R. Tsybin · Z. Benický · F. Gabrič · A. Von Meyer · E. Shigetaka · H. Van Kamp · C. Balcázar · S. Andriienko · R. Ramamurthy · H. Dragosavljević · S. Kirn · C. Valiente  …(+27)
  - **customer** | Chouhoud J. Cree · Speciose R. Kvedaravičius · Mammad W. Hoar · Moreese P. Sket · Bibi V. Brazaitis · Weijer X. Muraschko · Feetu L. Kandelis · Joghem L. Lisický · Wedderburn D. Puderbach · Ainamar S. Ball · Lémassou Z. Almeida · Qaisar G. Baggott  …(+53)
  - **first_last** | Harmanna Calland · Siu-Kei Shivute · Girigorio Łoziński · José de los Reyes McPharlin · Gommar Legrand · Alidou Reberšek · Hogne Cassanovschi · Zier Trofimenko · Ruba Simonaitytė · Sibille Paulíny · Rochford Salm-Reifferscheid-Raitz · Linditë Stevens  …(+20)
  - **Witness** | Haodong Drew · Luit Corrie · Beixuan Pastukh · Sheron Duž · Tang Vall-llobera · Parke Haylock · Melony Bagritsky · Chesa Metelko · Isedore Jeffress · Ewing Acheson · Adjani Moalla · Aqif Vošnjak  …(+47)


## Generator label coverage

GeoNames generator labels:
  -   2400  geography.location.country_code
  -   1200  geography.location.city
  -   1200  geography.location.region
  -   1200  geography.location.country
  -   1200  geography.location.continent
  -   1200  geography.location.us_state
  -   1200  geography.address.postal_code
  -   1200  geography.coordinate.latitude
  -   1200  geography.coordinate.longitude

Wikidata person generator labels:
  -   8000  identity.person.full_name
