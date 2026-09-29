package scopes.c;

import java.util.List;

class SortUtils {
    static int resolve(List<String> directionParams) {
        return directionParams.size();
        //     ^ d: src/main/java/scopes/c/SortUtils.java:6
        // status: directionParams → SortUtils.resolve.directionParams (local)
    }
}
