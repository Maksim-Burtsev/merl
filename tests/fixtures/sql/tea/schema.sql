-- #931: what SQL's own scoping makes hard for `d`.
CREATE SCHEMA tea;
CREATE SCHEMA stale;

CREATE TABLE tea.kettles (
  id integer PRIMARY KEY,
  cups integer
);

CREATE TABLE stale.kettles (
  id integer
);

CREATE TABLE teapots (
  id integer,
  kettle_id integer REFERENCES tea.kettles (id),
  leaves text
);

-- An overload, and a view declared again.
CREATE FUNCTION steep(cups integer) RETURNS integer AS $$
  SELECT cups * 2
$$ LANGUAGE sql;

CREATE FUNCTION steep(leaves text) RETURNS text AS $$
  SELECT leaves || '!'
$$ LANGUAGE sql;

CREATE VIEW brews AS SELECT id FROM teapots;
CREATE OR REPLACE VIEW brews AS SELECT id, leaves FROM teapots;

-- A parameter named like a table.
CREATE FUNCTION pour(teapots integer) RETURNS integer AS $$
  SELECT teapots + 1
  --     ^ d: tea/schema.sql:33
$$ LANGUAGE sql;

-- A string over lines holds a statement; a comment line opens with one.
INSERT INTO teapots (leaves) VALUES ('
CREATE TABLE ghosts (
  id integer
);
');
-- CREATE TABLE wraiths (id integer);

-- A backslash ends a standard string; in an E string it escapes the quote.
INSERT INTO teapots (leaves) VALUES ('C:\\');
CREATE TABLE trays (id integer);
INSERT INTO teapots (leaves) VALUES (E'it\'s');
CREATE TABLE saucers (id integer);

-- A table renamed declares its new name; a column renamed declares nothing.
ALTER TABLE saucers RENAME TO platters;
ALTER TABLE IF EXISTS ONLY tea.trays RENAME COLUMN id TO rims;
ALTER FUNCTION pour(integer) RENAME TO decant;
-- A rename to a name some CREATE declares yields to it, as on master: which counts is #1064.
ALTER TABLE cups RENAME TO trays;
