package calls;

class Use {
    Guard make(Ballot vote, Limits limits) {
    // ^ d: src/main/java/calls/Guard.java:3
        limits.admit("c, d", vote.ward().length());
        //     ^ d: src/main/java/calls/Limits.java:8
        //                        ^ d: src/main/java/calls/Ballot.java:4
        return new Guard("a", "b");
        //         ^ d: src/main/java/calls/Guard.java:8
    }

    Settings load(java.util.Map<String, Object> entries) {
        new Settings("settings.conf", 2);
        //  ^ d: src/main/java/calls/Settings.java:12
        return new Settings(entries);
        //         ^ d: picker src/main/java/calls/Settings.java:6, src/main/java/calls/Settings.java:9
    }
}
