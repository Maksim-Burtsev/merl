-- Old baskets are dumped to backup/*.sql first.
CREATE VIEW archived_baskets AS
  SELECT * FROM basket_lines;
