package getters;

public class Read {
    void show(Account account, Box<Meta> box) {
        account.getSpec().getDisplayName();
//                        ^ d: src/main/java/getters/Spec.java:7
        account.getMetadata().getName();
//                            ^ d: src/main/java/getters/Meta.java:4
        account.getSpec().getOwner().getDisplayName();
//                                   ^ d: src/main/java/getters/Owner.java:7
        account.getSpec().getDisplayName().trim();
//                                         ^ d: none
        this.account().getSpec().getDisplayName();
//                               ^ d: picker src/main/java/getters/Sender.java:4, src/main/java/getters/Owner.java:7, src/main/java/getters/Spec.java:7
        box.getItem().getName();
//                    ^ d: !jump
    }

    Account account() {
        return null;
    }
}
