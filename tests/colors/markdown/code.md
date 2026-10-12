# Code blocks

A fenced block in a language merl colours with tree-sitter:

```python
import re

def check(name: str) -> bool:
    """Says whether the name is fine."""
    return re.match(r"^[a-z_]+\d*$", name) is not None

print("%s has %d items" % (name, 3))
```

A fenced block without a language:

```
plain text in a fence
second line
```

A tilde fence:

~~~python
x = [1, 2, 3]
~~~

- A list item with code:

  ```python
  value = {"key": 1}
  ```
