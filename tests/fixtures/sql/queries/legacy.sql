-- MySQL quotes a name with a ` around it.
CREATE VIEW legacy_parcels AS
  SELECT * FROM parcel_log;
