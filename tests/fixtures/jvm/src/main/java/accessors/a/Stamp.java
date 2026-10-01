package accessors.a;

import lombok.Data;

@Data
public class Stamp {
    private String label;

    public String getLabel() {
        return label.trim();
    }
}
