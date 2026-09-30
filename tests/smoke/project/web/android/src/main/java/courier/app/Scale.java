package courier.app;

import courier.app.scan.Parcel;

class Scale {
    int weigh(Parcel parcel) {
        return parcel.grams();
    }
}
