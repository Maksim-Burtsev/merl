package scopes.c;

import java.util.List;

class Job extends Task {
    private final Scheduler scheduler = null;

    Job(String source) {
        super(source);
        //    ^ d: src/main/java/scopes/c/Job.java:8
        // status: source → Job.Job.source (local)
    }

    String f(String s) {
        scheduler.run(parentNameOf(s));
        // ^ d: src/main/java/scopes/c/Job.java:6
        // status: scheduler → Job.scheduler (via Job)
        //            ^ d: src/main/java/scopes/c/Job.java:31
        // status: parentNameOf → Job.parentNameOf (via Job)
        List.of(s).forEach(item -> scheduler.run(item));
        //                                       ^ d: src/main/java/scopes/c/Job.java:20
        for (String part : s.split(",")) {
            retry(part);
            //    ^ d: src/main/java/scopes/c/Job.java:22
            //^ d: src/main/java/scopes/c/Task.java:8
            // status: retry → Task.retry (via Task)
        }
        return s;
    }

    private static String parentNameOf(String item) {
        return item;
    }

    Runnable later(String label) {
        return new Runnable() {
            int tries = 0;

            @Override
            public void run() {
                tries++;
                //^ d: src/main/java/scopes/c/Job.java:37
                parentNameOf(label);
                //           ^ d: src/main/java/scopes/c/Job.java:35
            }
        };
    }
}
