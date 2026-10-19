-- #931: CTE and alias names, schema namesakes, and what declares nothing.
WITH teapots (id) AS (
  SELECT id FROM teapots
  --             ^ d: tea/schema.sql:14
)
SELECT id FROM teapots;
--             ^ d: tea/brew.sql:2

WITH kettles AS MATERIALIZED (
  SELECT 1 AS id
)
SELECT id FROM kettles;
--             ^ d: tea/brew.sql:9

SELECT teapots.id FROM tea.kettles AS teapots;
--     ^ d: tea/brew.sql:15

SELECT k.cups, p.leaves FROM tea.kettles k JOIN teapots p ON p.kettle_id = k.id;
--     ^ d: tea/brew.sql:18
--             ^ d: tea/brew.sql:18

SELECT * FROM stale.kettles, tea.kettles, kettles;
--            ^ d: tea/schema.sql:3
--                  ^ d: tea/schema.sql:10
--                                ^ d: tea/schema.sql:5
--                                        ^ d: picker tea/schema.sql:5, tea/schema.sql:10

SELECT * FROM (
  WITH teapots AS (SELECT 1 AS id)
  SELECT id FROM teapots
  --             ^ d: tea/brew.sql:29
) s, TEAPOTS;
--   ^ d: tea/schema.sql:14

SELECT steep(1), steep('x'), pour(2) FROM brews;
--     ^ d: picker tea/schema.sql:21, tea/schema.sql:25
--                 ^ d: picker tea/schema.sql:21, tea/schema.sql:25
--                           ^ d: tea/schema.sql:33
--                                        ^ d: picker tea/schema.sql:29, tea/schema.sql:30

SELECT * FROM ghosts, wraiths;
--            ^ d: none
--                    ^ d: none

SELECT * FROM trays, saucers WHERE leaves = E'it\'s';
--            ^ d: tea/schema.sql:48
--                   ^ d: tea/schema.sql:50
SELECT t.id FROM trays t;
--     ^ d: tea/brew.sql:48

SELECT decant(1) FROM platters, rims;
--     ^ d: tea/schema.sql:55
--                    ^ d: tea/schema.sql:53
--                              ^ d: none
