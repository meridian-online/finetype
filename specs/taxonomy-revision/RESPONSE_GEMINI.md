# **Semantic Type Classification and Transformation Contracts: A Strategic Analysis of the FineType Taxonomy in Global Data Ecosystems**

The contemporary data landscape is characterized by an unprecedented volume of semi-structured and tabular information, much of which is exchanged via plaintext formats such as Comma-Separated Values (CSV). While these formats offer universal accessibility, they suffer from a profound lack of semantic metadata, forcing analytical engines and data practitioners to rely on inference and heuristic-based type detection. This inference often fails to capture the true nature of the data, leading to a "semantic gap" where a column of strings is identified as generic text when it actually represents a specialized identifier, a locale-sensitive date, or a formatted financial value. The FineType project addresses this challenge by proposing a taxonomy of 163 semantic types categorized into six domains, functioning as "transformation contracts" that guarantee the success of specific cast expressions in vectorized engines like DuckDB. This report evaluates the frequency of real-world column types, benchmarks the FineType taxonomy against existing global standards, and identifies high-priority gaps to enhance the utility of semantic type detection for professional data analysts.

## **Frequency of Column Types in Real-World Tabular Datasets**

The analysis of frequency across diverse data sources reveals that while primitive types (integers, floats, booleans) form the underlying storage layer, semantic types define the operational utility of tabular datasets. In popular Kaggle datasets, which serve as a proxy for machine learning and exploratory data analysis trends, the most common columns include unique identifiers, categorical status labels, and precise physical measurements.1 For example, the "Credit Card Fraud Detection" dataset relies heavily on transaction IDs and principal component (PCA) transformed numeric values, while the "European Soccer Database" features complex X/Y coordinates for player positioning and categorical variables for team formations.2 These types represent a transition from simple "what" storage to "how" usage.  
Government open data portals (data.gov, data.gov.uk, and others) exhibit a high density of geospatial and institutional identifiers. Research into the data.gov catalog shows a preponderance of Federal Information Processing Series (FIPS) codes for states and counties, alongside Geographic Names Information System (GNIS) codes.3 These identifiers are critical because they allow for unambiguous joins across disparate public records, a task that would be impossible with natural language names alone. Similarly, public interest databases like those maintained by the World Bank utilize ISO3 country codes and unique machine-readable indicator codes (e.g., NY.GDP.MKTP.CD for GDP in current US dollars) to manage massive time-series arrays.4  
In the Enterprise and SaaS domain, schemas from platforms like Salesforce, Shopify, Stripe, and HubSpot reveal a deep reliance on business-logic types. Salesforce utilizes 15-character and 18-character case-sensitive IDs, which are fundamental to its relational model.6 Shopify exports frequently include "Handles" (URL-friendly slugs), SKUs (Stock Keeping Units), and Harmonized System (HS) codes for international taxation.8 Stripe’s API objects are rich in Unix epoch timestamps and nested address objects containing ISO 3166-1 alpha-2 country codes.9 HubSpot properties include specialized field types like "single checkbox" for booleans and "rich text" for formatted paragraphs, which can store up to 64 KB of data including images and links.11  
Scientific datasets in genomics and climate science introduce unique representational challenges. In these domains, columns often contain chemical formulas, units of measure (e.g., metric tons of $CO\_2$), and temperatures.2 The storage of such data often fluctuates between wide formats (where years or experiments are columns) and long formats (where they are row values), necessitating robust "tidy data" transformations.4

### **Observed Column Type Frequency and Ranking**

The following table catalogues the natural language descriptions of column types observed across these domains, ranked by their frequency and criticality for analytical workflows.

| Rank | Natural Language Description | Primary Sources | Frequency | FineType Equivalent |
| :---- | :---- | :---- | :---- | :---- |
| 1 | Unique Record Identifier (UUID, Alphanumeric ID) | Salesforce, Stripe, Kaggle | Very High | representation.text.alphanumeric\_id |
| 2 | ISO Country Code (Alpha-2/3) | World Bank, Stripe, Shopify | Very High | geography.location.country |
| 3 | Status / Priority / Stage Enum | Jira, Zendesk, HubSpot | High | representation.discrete.categorical |
| 4 | Email Address | HubSpot, Shopify, Zendesk | High | identity.person.email |
| 5 | Formatted Currency Amount (with symbol/commas) | Stripe, Shopify, Excel | High | **MISSING** |
| 6 | Date (Locale-Specific, e.g., MM/DD/YYYY) | Excel, data.gov, Kaggle | High | datetime.date.us\_slash |
| 7 | Alphanumeric Handle / Slug | Shopify, GitHub, Salesforce | High | representation.text.slug |
| 8 | Numeric Measurement with Unit | World Bank, Scientific Data | Medium | representation.scientific.unit\_of\_measure |
| 9 | Geospatial Coordinates (Lat/Long Pairs) | Kaggle, data.gov | Medium | geography.coordinate.pair |
| 10 | Unix Epoch Timestamp | Stripe, DuckDB exports | Medium | datetime.epoch.epoch\_seconds |
| 11 | Phone Number (Formatted) | Zendesk, Salesforce, HubSpot | Medium | geography.contact.phone\_number |
| 12 | Financial Instrument ID (ISIN, CUSIP) | World Bank, Financial Data | Low | identity.payment.isin |
| 13 | File Path / Extension | Salesforce, Kaggle | Low | representation.file.file\_extension |

## **Comparative Analysis of Type Taxonomies and Standards**

The FineType taxonomy, while comprehensive in its semantic depth, exists alongside several established standards that prioritize storage efficiency, validation constraints, or web-scale interoperability. Benchmarking FineType against these tools highlights significant overlaps and critical gaps.

### **Pandas and Numerical Inference**

The pandas infer\_dtype function is perhaps the most widely used tool for automated type detection in Python. However, its taxonomy is remarkably shallow, focusing on storage-level primitives rather than semantic meaning. It returns string labels such as string, bytes, floating, integer, boolean, and the notoriously ambiguous mixed.12 In the presence of missing values (NaN), pandas historically converts integer columns to float64, a behavior that analysts find highly frustrating as it mutates "1" into "1.0," potentially breaking joins on ID columns.13 FineType’s advantage lies in its ability to look past the storage layer; where pandas sees an object or mixed type, FineType can identify an identity.person.email or a technology.internet.ip\_v4.

### **Great Expectations and dbt**

Great Expectations and the dbt-expectations package focus on data quality and validation rather than just classification. Their "types" are often defined as expectations, such as expect\_column\_values\_to\_be\_of\_type or expect\_column\_values\_to\_match\_regex.15 While FineType classifies the data, Great Expectations enforces the classification. FineType’s taxonomy is significantly more granular than the core types supported by Great Expectations (numerics, strings, booleans, dates).17 However, dbt-expectations includes high-value tests for JSON schema adherence and recent data checks (expect\_row\_values\_to\_have\_recent\_data), which are structural rather than semantic.16

### **Schema.org and Linked Data**

Schema.org provides a massive hierarchy of 827 types and 14 core datatypes, but its primary purpose is web-scale search engine optimization and knowledge graph construction.19 It includes extremely specific types like Volcano or MedicalCondition, but for tabular data, it relies on Dataset, DataDownload, and Table properties.20 FineType operates at a more practical "column-level" for analysts, whereas Schema.org focuses on "record-level" or "entity-level" ontology. A significant gap in FineType is the lack of "Property Sync" or "Lookup" types found in CRM metadata, which indicate relationship logic between tables.7

### **Frictionless Data and CSVW**

The Frictionless Data (Table Schema) and W3C's CSV on the Web (CSVW) standards are the most direct competitors to FineType's philosophy. Frictionless defines types like year, yearmonth, geopoint, and duration.23 It also supports "bareNumber" constraints, which allow implementors to strip leading/trailing non-numeric characters like currency symbols or percentage signs.24 This is a critical feature missing from the current FineType taxonomy. CSVW builds upon XML Schema datatypes, offering specialized numeric types like unsignedInt and nonNegativeInteger, as well as hexBinary and base64Binary.25 FineType’s "technology" domain is more modern, covering IP addresses and MAC addresses that these older standards lack.

### **SaaS and LLM Type Systems**

SaaS platforms like Jira, Zendesk, and Stripe use internal type systems tailored to their API requirements. Jira, for instance, distinguishes between user (Atlassian account ID), group, and cascading select (hierarchical dropdowns).26 Zendesk utilizes "lookup relationship" fields to connect tickets to custom objects like "assets" or "contracts".28 OpenAI's structured outputs and function calling support a strict subset of JSON Schema, including string, number, boolean, integer, object, and array.30 FineType could bridge the gap between LLM-generated JSON and enterprise SQL by providing the regex patterns needed for LLMs to strictly follow formatted semantic types.

### **Excel and Google Sheets**

Spreadsheet software utilizes "Number Format Categories" that users expect as a standard. These include Currency, Accounting, Fraction, and Scientific.32 A key feature of these tools is locale-awareness; the decimal separator and thousands separator are determined by the document's locale, a major source of frustration when data is moved between regions.32 FineType’s taxonomy currently treats decimals and si\_numbers generically, ignoring the high-value "Accounting" format where negative numbers are often enclosed in parentheses, e.g., $(50.00)$.34

## **Analyst Frustrations with Data Type Detection and Ambiguity**

The research into practitioner communities (Reddit, Stack Overflow, and technical blogs) reveals a landscape fraught with "silent data corruption" and manual cleaning labor. The "NaN-to-Float" conversion in pandas is the most cited grievance, as it turns categorical integers into floats, creating aesthetic noise and breaking referential integrity.13 Analysts frequently express a desire for tools that "search before creating," reflecting the high cost of duplicate records in CRM systems like Dynamics 365, where 10–30% of data is often duplicated due to uncoordinated manual entry or bulk imports without pre-checks.36  
Date formatting remains a primary source of "hell" for analysts. The ambiguity between US (MM/DD/YYYY) and International (DD/MM/YYYY) formats often leads to dates being swapped or misread without throwing an error.33 Practitioners often "wish" for a tool that can not only detect the format but also validate it using functions like ISDATE() in Google Sheets or ISNUMBER() in Excel.33 The "Object trap" in pandas—where a single malformed string forces an entire column of integers into a generic object type—is a significant pain point, requiring the use of pd.to\_numeric(errors='coerce') which can lead to data loss.39  
Furthermore, the scale of modern data poses a challenge. Analysts dumping CSVs with "hundreds of columns and millions of rows" find that existing inference methods like convert\_dtypes() are often ineffective and slow.38 There is a clear demand for "read-time validation" that can handle high-dimensional data without requiring exhaustive manual casting.38

## **High-Value DuckDB Transformations via Semantic Types**

The value proposition of FineType as a "transformation contract" is realized through its ability to map semantic types to efficient DuckDB SQL expressions. The research identifies several areas where precise classification unlocks significant value by automating manual cleaning.

### **Date and Timestamp Parsing**

The most frequent transformation task involves converting variety-formatted strings into standard ISO timestamps. DuckDB’s strptime function is the workhorse here, but it requires a perfectly matching format string.41 FineType’s 46 datetime types map directly to these format strings. For instance, datetime.date.us\_slash maps to strptime(value, '%m/%d/%Y'). Identifying a column as a 14-digit numeric timestamp (e.g., 20250119130123\) allows for a high-value chain: CAST(Timestamp AS VARCHAR).strptime('%Y%m%d%H%M%S').42 This saves analysts from the trial-and-error process of identifying the correct pattern specifiers like %I for 12-hour clocks or %p for AM/PM.41

### **Numeric and Currency Cleaning**

Financial data is rarely "clean" enough for a direct CAST(col AS DECIMAL). The presence of dollar signs, commas for thousands separators, and parentheses for negative values requires string manipulation.44 A semantic type for "Formatted Currency" would enable an automated DuckDB transform using replace(): SUM(CAST(REPLACE(REPLACE(price, '$', ''), ',', '') AS DECIMAL(18,2))).44 Similarly, percentage strings like "95%" require stripping the suffix and dividing by 100 to reach a decimal representation.24

### **JSON and Nested Object Unpacking**

Modern data often arrives with JSON strings embedded in columns (e.g., Stripe's metadata or Salesforce's picklistValues).9 DuckDB provides powerful JSON extraction operators (-\> and \-\>\>) and the json\_transform function.47 If FineType identifies a column as representation.text.json\_path or a specific technical object, it can trigger a "JSON to Struct" transformation: from\_json(col, '{"key": "VARCHAR"}'), which converts the string into a natively queryable DuckDB STRUCT.48 This is significantly more efficient than extracting values one-by-one.48

### **Identifier Optimization**

DuckDB supports specialized storage for UUIDs (128-bit).49 By identifying a column as a UUID rather than a generic string, FineType enables a cast to the UUID type, which optimizes join performance and reduces memory overhead compared to storing the same value as a 36-character string.49

## **Critical Evaluation of the FineType Taxonomy**

Applying the research findings to FineType's specific taxonomic structure reveals several areas for improvement, particularly in the categorization of financial and organizational data.

### **1\. Financial Identifiers and the Identity Domain**

The placement of ISIN, CUSIP, SEDOL, and LEI under identity.payment is architecturally problematic. These codes are "Securities Identifiers" used for global market trading, reporting, clearing, and settlement; they are not payment instruments.50 The LEI (Legal Entity Identifier) is a 20-character code specifically for identifying organizations involved in financial transactions, not the transactions themselves.52 Analysts would logically look for these under a "Securities" or "Finance" category rather than "Payment."

### **2\. The Value of the Scientific Category**

The representation.scientific category is not over-engineered. On the contrary, it is essential for the integrity of physical and experimental data. The distinction between a temperature and a generic float is vital; without knowing the scale (Celsius vs. Fahrenheit) or the unit, the numeric value is meaningless. Public health and environmental datasets (WHO, World Bank) rely heavily on these units of measure to ensure that emissions or vaccination rates are comparable across regions.4

### **3\. Datetime Domain Over-Specification**

While 46 types may seem excessive, they are justified by the requirement for "transformation contracts." In DuckDB, a single character difference in a format string (e.g., %y for two-digit years vs %Y for four-digit years) will cause a strptime failure.41 The high level of specificity allows FineType to act as a robust pre-parser, ensuring that the selected transformation is mathematically guaranteed to succeed. Reducing the number of types would introduce the very ambiguity that the project aims to solve.

### **4\. Missing Business and Structural Types**

The taxonomy currently lacks several high-frequency types observed in SaaS exports:

* **Formatted Currency Amount:** As noted, identifying the symbol is not enough; the entire pattern (e.g., \-$1,234.56) must be recognized to enable cleaning.  
* **Status/Stage Enum:** Fundamental to CRM and project management.54  
* **Lookup/Reference ID:** Crucial for understanding foreign key relationships in systems like Salesforce or Zendesk.7  
* **ISO Country Code (Alpha-2/3):** The current country type likely targets country names, but code-based identifiers are more common for data integration.4

### **5\. Email Redundancy**

The existence of identity.person.email and technology.internet.email reflects a dual reality in data modeling. From an identity perspective, an email represents a person; from a technical perspective, it is a unique identifier string.56 While the validation regex is identical, the context differs. However, for a classification tool, this redundancy can be confusing. It is recommended to consolidate these into a single semantic type under a more generic contact or internet category.

## **Strategic Recommendations and Taxonomy Refinement**

The following findings represent high-priority updates to the FineType taxonomy to align with industry standards and analyst needs.

### **High-Priority Gaps (Additions)**

| Type | Source | Frequency | FineType Equivalent | Analyst Value | DuckDB Transform |
| :---- | :---- | :---- | :---- | :---- | :---- |
| **Formatted Amount** | Stripe, Shopify, Excel | High | **MISSING** | Financial math requires symbol/comma removal.44 | regexp\_replace(val, '\[^0-9.-\]', '', 'g')::DECIMAL |
| **Status/Priority** | Jira, Zendesk, Shopify | High | **MISSING** | Core for funnel and workflow analysis.54 | val::VARCHAR (categorical check) |
| **ISO Country Code** | World Bank, Stripe | High | **MISSING** | Standardized joins for geographic enrichment.4 | val::VARCHAR (ISO-3166 check) |
| **Reference ID** | Salesforce, Zendesk | High | **MISSING** | Identifies foreign keys and relational links.7 | val::VARCHAR (prefix validation) |
| **FIPS/GNIS Code** | data.gov | Medium | **MISSING** | Essential for US public sector data integration.3 | val::INTEGER (padded) |
| **Rich Text/HTML** | HubSpot, Zendesk | Medium | **MISSING** | Requires sanitization before plain-text analysis.11 | regexp\_replace(val, '\<\[^\>\]\*\>', '', 'g') |
| **Scientific Notation** | Excel, Kaggle, DuckDB | Low | scientific\_notation | Prevents precision loss in large/small numbers.35 | val::DOUBLE (E-notation) |

### **Removal and Consolidation Candidates**

* **identity.person.email**: Consolidate with technology.internet.email into a single contact.email type. The technical contract is identical.56  
* **representation.text.slug**: Consider moving to technology.internet.slug as it is primarily a web-routing identifier (Shopify Handles).8  
* **identity.person.username**: Often functionally identical to alphanumeric\_id or email in modern systems. If it lacks a specific format, its value as a separate type is low.

### **Rename Suggestions**

* **identity.payment**: Rename to finance.payment. This allows for a more natural parent for payment instruments like credit cards and IBANs.  
* **identity.person.age**: Consider moving to representation.numeric.age. While it describes a person, it is mathematically a discrete integer.  
* **representation.numeric.si\_number**: Rename to representation.numeric.formatted\_size (e.g., 10 MB, 1 GB) to align with analyst terminology.58

### **Structure Changes: Domain Reorganization Proposal**

To improve discoverability and logical flow, the following reorganization is proposed:

1. **Identity & Entity:** (Person, Organization, Medical). Focuses on "Who."  
2. **Finance & Securities:** (Payment, Securities, Currency). Focuses on "Value and Assets." This addresses the misclassification of ISIN/CUSIP.  
3. **Temporal:** (Dates, Times, Durations, Epochs). Focuses on "When." This keeps the 46 precise datetime types together.  
4. **Geographic:** (Addresses, Locations, Coordinates, Transportation). Focuses on "Where."  
5. **Technical & Web:** (Internet, Development, Hardware, Cryptographic). Focuses on "How it communicates."  
6. **Representational & Discrete:** (Boolean, File, Numeric, Scientific, Text, Discrete Enums). Focuses on "How it is formatted."

This structure separates the "subject" of the data (Person, Finance) from the "format" of the data (Technical, Representational), which is more intuitive for analysts building schemas.

## **Conclusion: The Operational Impact of Semantic Contracts**

The transition from generic data inference to semantic "transformation contracts" represents a significant evolution in data engineering. The research into analyst frustrations and SaaS schema complexity demonstrates that the primary bottleneck in data analysis is not storage, but the labor-intensive process of cleaning and casting plaintext data into usable types. FineType’s taxonomy, by providing 163 distinct contracts, offers a pathway to automating this labor. However, to achieve full maturity, the taxonomy must expand its coverage of formatted business types—specifically currency amounts and status enums—and correct the misclassification of financial securities identifiers. By aligning the taxonomy with the practical needs of vectorized engines like DuckDB and the structural realities of CRM and ERP exports, FineType can provide the foundational metadata required for the next generation of automated data pipelines. The ultimate goal is a system where the "semantic gap" is closed at the moment of ingestion, ensuring that every column is not just an array of characters, but a trusted, castable, and operationally ready asset.

#### **Works cited**

1. Most popular Kaggle datasets, accessed on March 2, 2026, [https://www.kaggle.com/datasets/danyocean/most-popular-kaggle-datasets](https://www.kaggle.com/datasets/danyocean/most-popular-kaggle-datasets)  
2. 10 Most Popular Datasets On Kaggle, accessed on March 2, 2026, [https://www.kaggle.com/discussions/general/260690](https://www.kaggle.com/discussions/general/260690)  
3. Dataset \- Catalog \- Data.gov, accessed on March 2, 2026, [https://catalog.data.gov/](https://catalog.data.gov/)  
4. World Bank Datasets Explained: From Download to Analysis \- Fun With Data, accessed on March 2, 2026, [https://www.funwithdata.ca/resources/data-analytics-toolbox/world-bank-datasets-explained-from-download-to-analysis](https://www.funwithdata.ca/resources/data-analytics-toolbox/world-bank-datasets-explained-from-download-to-analysis)  
5. World Bank Open Data | Data, accessed on March 2, 2026, [https://data.worldbank.org/](https://data.worldbank.org/)  
6. Field Schema Details Available Using $ObjectType | Visualforce Developer Guide, accessed on March 2, 2026, [https://developer.salesforce.com/docs/atlas.en-us.pages.meta/pages/pages\_variables\_global\_objecttype\_schema\_fields\_reference.htm](https://developer.salesforce.com/docs/atlas.en-us.pages.meta/pages/pages_variables_global_objecttype_schema_fields_reference.htm)  
7. Field Types | Object Reference for the Salesforce Platform ..., accessed on March 2, 2026, [https://developer.salesforce.com/docs/atlas.en-us.object\_reference.meta/object\_reference/field\_types.htm](https://developer.salesforce.com/docs/atlas.en-us.object_reference.meta/object_reference/field_types.htm)  
8. Exporting and importing inventory with a CSV ... \- Shopify Help Center, accessed on March 2, 2026, [https://help.shopify.com/en/manual/products/inventory/setup/inventory-csv](https://help.shopify.com/en/manual/products/inventory/setup/inventory-csv)  
9. The Customer object | Stripe API Reference, accessed on March 2, 2026, [https://docs.stripe.com/api/customers/object](https://docs.stripe.com/api/customers/object)  
10. The Subscription object | Stripe API Reference, accessed on March 2, 2026, [https://docs.stripe.com/api/subscriptions/object](https://docs.stripe.com/api/subscriptions/object)  
11. Understand property field types in HubSpot, accessed on March 2, 2026, [https://knowledge.hubspot.com/properties/property-field-types-in-hubspot](https://knowledge.hubspot.com/properties/property-field-types-in-hubspot)  
12. pandas.api.types.infer\_dtype — pandas 3.0.1 documentation, accessed on March 2, 2026, [https://pandas.pydata.org/docs/reference/api/pandas.api.types.infer\_dtype.html](https://pandas.pydata.org/docs/reference/api/pandas.api.types.infer_dtype.html)  
13. Pandas Data Types and Performance Considerations \- Data Science in Practice, accessed on March 2, 2026, [https://notes.dsc80.com/content/02/data-types.html](https://notes.dsc80.com/content/02/data-types.html)  
14. Has anyone found a solution for Pandas bad data type inference? \- Stack Overflow, accessed on March 2, 2026, [https://stackoverflow.com/questions/56917460/has-anyone-found-a-solution-for-pandas-bad-data-type-inference](https://stackoverflow.com/questions/56917460/has-anyone-found-a-solution-for-pandas-bad-data-type-inference)  
15. Expectations overview \- Great Expectations documentation, accessed on March 2, 2026, [https://docs.greatexpectations.io/docs/cloud/expectations/expectations\_overview/](https://docs.greatexpectations.io/docs/cloud/expectations/expectations_overview/)  
16. dbt-expectations: What it is and how to use it to find data quality issues | Metaplane, accessed on March 2, 2026, [https://www.metaplane.dev/blog/dbt-expectations](https://www.metaplane.dev/blog/dbt-expectations)  
17. What data types does Great Expectations support for validation?, accessed on March 2, 2026, [https://discourse.greatexpectations.io/t/what-data-types-does-great-expectations-support-for-validation/744](https://discourse.greatexpectations.io/t/what-data-types-does-great-expectations-support-for-validation/744)  
18. dbt\_expectations \- dbt \- Package hub, accessed on March 2, 2026, [https://hub.getdbt.com/metaplane/dbt\_expectations/latest](https://hub.getdbt.com/metaplane/dbt_expectations/latest)  
19. Schema.org, accessed on March 2, 2026, [https://schema.org/docs/schemas.html](https://schema.org/docs/schemas.html)  
20. Dataset \- Schema.org Type, accessed on March 2, 2026, [https://schema.org/Dataset](https://schema.org/Dataset)  
21. Data and Datasets overview \- Schema.org, accessed on March 2, 2026, [https://schema.org/docs/data-and-datasets.html](https://schema.org/docs/data-and-datasets.html)  
22. Table \- Schema.org Type, accessed on March 2, 2026, [https://schema.org/Table](https://schema.org/Table)  
23. Table Schema • frictionless \- rOpenSci, accessed on March 2, 2026, [https://docs.ropensci.org/frictionless/articles/table-schema.html](https://docs.ropensci.org/frictionless/articles/table-schema.html)  
24. Table Schema | Data Package Standard, accessed on March 2, 2026, [https://datapackage.org/standard/table-schema/](https://datapackage.org/standard/table-schema/)  
25. csvw.datatypes — csvw 3.7.0 documentation, accessed on March 2, 2026, [https://csvw.readthedocs.io/en/stable/datatypes.html](https://csvw.readthedocs.io/en/stable/datatypes.html)  
26. Jira custom field type \- Developer, Atlassian, accessed on March 2, 2026, [https://developer.atlassian.com/platform/forge/manifest-reference/modules/jira-custom-field-type/](https://developer.atlassian.com/platform/forge/manifest-reference/modules/jira-custom-field-type/)  
27. Field types you can create as a Jira admin \- Atlassian Support, accessed on March 2, 2026, [https://support.atlassian.com/jira-cloud-administration/docs/field-types-you-can-create-as-a-jira-admin/](https://support.atlassian.com/jira-cloud-administration/docs/field-types-you-can-create-as-a-jira-admin/)  
28. Adding custom ticket fields to your tickets and forms \- Zendesk help, accessed on March 2, 2026, [https://support.zendesk.com/hc/en-us/articles/4408883152794-Adding-custom-ticket-fields-to-your-tickets-and-forms](https://support.zendesk.com/hc/en-us/articles/4408883152794-Adding-custom-ticket-fields-to-your-tickets-and-forms)  
29. How to relate Zendesk custom objects to tickets: A complete guide \- eesel AI, accessed on March 2, 2026, [https://www.eesel.ai/blog/zendesk-custom-objects-relate-to-tickets](https://www.eesel.ai/blog/zendesk-custom-objects-relate-to-tickets)  
30. How to use structured outputs with Azure OpenAI in Microsoft Foundry Models, accessed on March 2, 2026, [https://learn.microsoft.com/en-us/azure/ai-foundry/openai/how-to/structured-outputs?view=foundry-classic](https://learn.microsoft.com/en-us/azure/ai-foundry/openai/how-to/structured-outputs?view=foundry-classic)  
31. Diving Deeper with Structured Outputs \- Towards Data Science, accessed on March 2, 2026, [https://towardsdatascience.com/diving-deeper-with-structured-outputs-b4a5d280c208/](https://towardsdatascience.com/diving-deeper-with-structured-outputs-b4a5d280c208/)  
32. Format numbers in a spreadsheet \- Computer \- Google Docs Editors Help, accessed on March 2, 2026, [https://support.google.com/docs/answer/56470?hl=en\&co=GENIE.Platform%3DDesktop](https://support.google.com/docs/answer/56470?hl=en&co=GENIE.Platform%3DDesktop)  
33. Format your Google Sheets and Excel files \- Help Center \- Databox, accessed on March 2, 2026, [https://help.databox.com/format-your-google-sheets-and-excel-files](https://help.databox.com/format-your-google-sheets-and-excel-files)  
34. Implementing Custom Number Formats in Google Sheets (Versus Excel's Number Format System) \- Statology, accessed on March 2, 2026, [https://www.statology.org/implementing-custom-number-formats-in-google-sheets-versus-excels-number-format-system/](https://www.statology.org/implementing-custom-number-formats-in-google-sheets-versus-excels-number-format-system/)  
35. A Comprehensive Google Sheets Custom Number Format Guide \- SheetWhiz, accessed on March 2, 2026, [https://www.sheetwhiz.com/post/google-sheets-custom-number-format-guide](https://www.sheetwhiz.com/post/google-sheets-custom-number-format-guide)  
36. How To Fix Duplicate Data In Dynamics 365? \- SoftArt Solutions, accessed on March 2, 2026, [https://softartsolutionsinc.com/dynamics-365/how-to-fix-duplicate-data-in-dynamics-365/](https://softartsolutionsinc.com/dynamics-365/how-to-fix-duplicate-data-in-dynamics-365/)  
37. Why Modern CRM Leaders Need a Duplicate Detection Strategy in Microsoft Dynamics 365, accessed on March 2, 2026, [https://msdynamicsworld.com/story/why-modern-crm-leaders-need-duplicate-detection-strategy-microsoft-dynamics-365](https://msdynamicsworld.com/story/why-modern-crm-leaders-need-duplicate-detection-strategy-microsoft-dynamics-365)  
38. How do you guys handle pandas and its sh\*tty data type inference : r ..., accessed on March 2, 2026, [https://www.reddit.com/r/Python/comments/12m2gn8/how\_do\_you\_guys\_handle\_pandas\_and\_its\_shtty\_data/](https://www.reddit.com/r/Python/comments/12m2gn8/how_do_you_guys_handle_pandas_and_its_shtty_data/)  
39. Wrong datatype detection for csv files in pandas \- Stack Overflow, accessed on March 2, 2026, [https://stackoverflow.com/questions/66741506/wrong-datatype-detection-for-csv-files-in-pandas](https://stackoverflow.com/questions/66741506/wrong-datatype-detection-for-csv-files-in-pandas)  
40. How to deal with errors of defining data types in pandas' read\_csv ()?, accessed on March 2, 2026, [https://datascience.stackexchange.com/questions/68697/how-to-deal-with-errors-of-defining-data-types-in-pandas-read-csv](https://datascience.stackexchange.com/questions/68697/how-to-deal-with-errors-of-defining-data-types-in-pandas-read-csv)  
41. Date Format – DuckDB \- AiDocZh, accessed on March 2, 2026, [https://aidoczh.com/duckdb/docs/archive/0.9/sql/functions/dateformat.html](https://aidoczh.com/duckdb/docs/archive/0.9/sql/functions/dateformat.html)  
42. Exploring DuckDB \- Part 2 (Dates, Times, CLI) \- Mohit Sindhwani, accessed on March 2, 2026, [https://notepad.onghu.com/2025/exploring-duckdb-part2/](https://notepad.onghu.com/2025/exploring-duckdb-part2/)  
43. Date Format Functions \- DuckDB, accessed on March 2, 2026, [https://duckdb.org/docs/stable/sql/functions/dateformat](https://duckdb.org/docs/stable/sql/functions/dateformat)  
44. How to Remove Dollar Signs from Prices in SQL and Convert to Decimal \- YouTube, accessed on March 2, 2026, [https://www.youtube.com/watch?v=GiHFPAFCKD4](https://www.youtube.com/watch?v=GiHFPAFCKD4)  
45. TYPE CONVERSION: How do you convert a string to a decimal? \- Stack Overflow, accessed on March 2, 2026, [https://stackoverflow.com/questions/77817672/type-conversion-how-do-you-convert-a-string-to-a-decimal](https://stackoverflow.com/questions/77817672/type-conversion-how-do-you-convert-a-string-to-a-decimal)  
46. Document | Object Reference for the Salesforce Platform, accessed on March 2, 2026, [https://developer.salesforce.com/docs/atlas.en-us.object\_reference.meta/object\_reference/sforce\_api\_objects\_document.htm](https://developer.salesforce.com/docs/atlas.en-us.object_reference.meta/object_reference/sforce_api_objects_document.htm)  
47. JSON Overview \- DuckDB, accessed on March 2, 2026, [https://duckdb.org/docs/stable/data/json/overview](https://duckdb.org/docs/stable/data/json/overview)  
48. JSON Processing Functions \- DuckDB, accessed on March 2, 2026, [https://duckdb.org/docs/stable/data/json/json\_functions](https://duckdb.org/docs/stable/data/json/json_functions)  
49. Numeric Types \- DuckDB, accessed on March 2, 2026, [https://duckdb.org/docs/stable/sql/data\_types/numeric](https://duckdb.org/docs/stable/sql/data_types/numeric)  
50. Identifiers (ISIN, CUSIP, FIGI) \- TradingView, accessed on March 2, 2026, [https://www.tradingview.com/support/solutions/43000734977-identifiers-isin-cusip-figi/](https://www.tradingview.com/support/solutions/43000734977-identifiers-isin-cusip-figi/)  
51. Difference Between ISIN and CUSIP \- ISIN \- International Securities Identification Number, accessed on March 2, 2026, [https://www.isin.net/difference-between-isin-and-cusip/](https://www.isin.net/difference-between-isin-and-cusip/)  
52. Company codes \- CUSIP, SEDOL, ISIN…. What do they mean and how can you use them in our Library resources? \- Cranfield University Blogs, accessed on March 2, 2026, [https://blogs.cranfield.ac.uk/library/company\_codes/](https://blogs.cranfield.ac.uk/library/company_codes/)  
53. GLOBAL SECURITIES IDENTIFIERS ISIN, CUSIP, AND LEI, accessed on March 2, 2026, [https://applyforisin.com/global-securities-and-corporate-identifiers/](https://applyforisin.com/global-securities-and-corporate-identifiers/)  
54. Exporting your user management information as a CSV file \- Shopify Help Center, accessed on March 2, 2026, [https://help.shopify.com/en/manual/your-account/users/csv-exports](https://help.shopify.com/en/manual/your-account/users/csv-exports)  
55. Master Jira Field Types: A Guide for IT Consultants \- Getint, accessed on March 2, 2026, [https://www.getint.io/blog/jira-field-types-guide-it-consultants](https://www.getint.io/blog/jira-field-types-guide-it-consultants)  
56. Prevent duplicate contacts when email addresses change in your CRM Integrations, accessed on March 2, 2026, [https://support.dotdigital.com/en/articles/11560234-prevent-duplicate-contacts-when-email-addresses-change-in-your-crm-integrations](https://support.dotdigital.com/en/articles/11560234-prevent-duplicate-contacts-when-email-addresses-change-in-your-crm-integrations)  
57. Literal Types \- DuckDB, accessed on March 2, 2026, [https://duckdb.org/docs/stable/sql/data\_types/literal\_types](https://duckdb.org/docs/stable/sql/data_types/literal_types)  
58. Text Functions \- DuckDB, accessed on March 2, 2026, [https://duckdb.org/docs/stable/sql/functions/text](https://duckdb.org/docs/stable/sql/functions/text)
