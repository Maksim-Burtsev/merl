package courier.app;

import courier.app.scan.Parcel;
import lombok.Getter;

class Scale {
    int weigh(Parcel parcel) {
        return parcel.grams();
    }

    @Getter
    private int limit = 30;

    boolean fits(Scale scale, Parcel parcel) {
        return parcel.grams() <= scale.getLimit();
    }
}
