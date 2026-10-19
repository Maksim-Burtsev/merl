package quern;

public interface Turnable {
    default int crank() {
        return 3;
    }
}
