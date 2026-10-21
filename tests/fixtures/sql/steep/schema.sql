-- #1126: columns a table takes from another, output names, quoted case, nested comments.
CREATE TABLE samovars (
  id integer PRIMARY KEY,
  spout integer,
  brim text
);

CREATE TABLE urns (
  handle integer
) INHERITS (samovars);

CREATE TABLE cozies (LIKE samovars INCLUDING ALL, knit text);

CREATE VIEW brewings AS SELECT id, spout AS strength FROM samovars;

CREATE TABLE "Kettle" (
  "Lid" integer,
  lid integer
);

-- In a function of LANGUAGE sql a column takes precedence over a parameter of its name.
CREATE FUNCTION infuse(spout integer) RETURNS integer AS $$
  SELECT spout FROM samovars
  --     ^ d: steep/schema.sql:22; want steep/schema.sql:4 (#1225)
$$ LANGUAGE sql;

/* A block comment nests: /* this one closes first */
CREATE TABLE phantoms (id integer);
*/

CREATE FUNCTION decoct() RETURNS text AS $body$
  SELECT $$
CREATE TABLE mirages (id integer);
$$
$body$ LANGUAGE sql;
