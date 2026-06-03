.print == load ==
LOAD '/Users/hugh/github/meridian-online/finetype/target/release/finetype.duckdb_extension';

.print == ft_version ==
SELECT ft_version();

.print == ft_infer scalar ==
SELECT ft_infer('jane.doe@company.co.uk') AS t;

.print == ft_detail scalar (utility alias) ==
SELECT ft_detail('jane.doe@company.co.uk') AS d;

.print == build people table ==
CREATE TABLE people AS
  SELECT * FROM (VALUES
    ('jane.doe@company.co.uk', '+44 20 7946 0958', {'city': 'London'}),
    ('john.smith@example.org', '0117 496 0123',    {'city': 'Bristol'}),
    ('not-an-email',           'not-a-phone',      {'city': 'Leeds'})
  ) AS t(email, phone, addr);

.print == ft_profile(table) — table macro, one row per column ==
SELECT * FROM ft_profile('people');

.print == ft_profile(list(email)) — scalar still resolves in projection ==
SELECT ft_profile(list(email)) AS p FROM people;

.print == ft_validate_text inline (valid email) ==
SELECT ft_validate_text('jane.doe@company.co.uk', '{"type":"string","pattern":"^[^@]+@[^@]+\\.[^@]+$"}') AS r;

.print == ft_validate_text inline (bad email) ==
SELECT ft_validate_text('not-an-email', '{"type":"string","pattern":"^[^@]+@[^@]+\\.[^@]+$"}') AS r;

.print == ft_validate inline schema ==
SELECT * FROM ft_validate('people', '{"properties":{"email":{"type":"string","pattern":"^[^@]+@[^@]+\\.[^@]+$"},"phone":{"type":"string","pattern":"^\\+?[0-9().+ -]{7,}$"}}}');

.print == ft_validate file schema ==
SELECT * FROM ft_validate('people', 'people_schema.json');
