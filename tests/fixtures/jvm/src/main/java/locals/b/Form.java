package locals.b;

import static org.apache.commons.lang3.StringUtils.isBlank;

class Form {
    boolean empty(String s) {
        return isBlank(s);
        //     ^ d: none
        // status: no definition for isBlank
    }
}
