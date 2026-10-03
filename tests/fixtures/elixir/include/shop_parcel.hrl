-record(parcel, {ref, grams = 0}).

-record(crate,
        {label,
         parcels = [] :: [term()],
         sealed = false}).

-define(CRATE_LIMIT, 12).
