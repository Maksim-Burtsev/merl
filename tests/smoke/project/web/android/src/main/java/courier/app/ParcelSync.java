package courier.app;

import java.util.List;

/** Sends the parcels scanned offline once the phone is back online. */
public class ParcelSync {
    private final Dispatcher dispatcher = new Dispatcher();

    public int send(List<String> parcelCodes) {
        parcelCodes.forEach(code -> dispatcher.run(code));
        return parcelCodes.size();
    }
}
