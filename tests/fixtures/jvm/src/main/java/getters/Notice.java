package getters;

import java.util.HashMap;
import java.util.Map;

public class Notice {
    private final Map<String, Object> attributes;

    Notice(Map<String, Object> attributes) {
        this.attributes = attributes;
    }

    public static NoticeBuilder builder() {
        return new NoticeBuilder();
    }

    public static class NoticeBuilder {
        private final Map<String, Object> attributes = new HashMap<>();

        public NoticeBuilder attribute(String key, Object value) {
            attributes.put(key, value);
            return this;
        }
    }
}
