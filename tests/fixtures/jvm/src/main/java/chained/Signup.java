package chained;

public class Signup {
    void register(User user, Address a) {
        user.setAddress(a).setName("n");
//                         ^ d: picker src/main/java/chained/Address.java:9, src/main/java/chained/User.java:10; want src/main/java/chained/User.java:10 (#1278)
        user.getAddress().getName();
//                        ^ d: src/main/java/chained/Address.java:9
    }
}
