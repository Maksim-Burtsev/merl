package courier.app.scan;

/** A parcel as the courier scans it. */
public record Parcel(
        String code,
        int grams) {}
