"""Every construct `f` folds in Python, and the cases that once broke it."""

import os  # f: 3-8
import sys  # f: 3-8
from typing import (  # f: 3-8
    Any,
    Optional,
)

PATTERN = r'(?:\s+["\'](?P<title>.*?)["\'])?'


@decorator
class Store:  # f: 14-87
    """
    A docstring the class folds from inside.
    """

    def __init__(  # f: 19-23
        self, path: str, size: int  # f: 19-23
    ) -> None:  # f: 19-23
        self.path = path  # f: 19-23
        self.size = size  # f: 19-23

    async def read(self, key: str) -> Optional[bytes]:  # f: 25-47
        if key in self.cache:  # f: 26-27
            return self.cache[key]  # f: 25-47
        elif key.startswith("_"):  # f: 28-29
            return None  # f: 25-47
        else:  # f: 30-31
            value = await self.load(key)  # f: 25-47
        for part in key.split("/"):  # f: 32-33
            pass  # f: 25-47
        else:  # f: 34-35
            part = None  # f: 25-47
        while True:  # f: 36-37
            break  # f: 25-47
        with open(self.path) as f:  # f: 38-39
            data = f.read()  # f: 25-47
        try:  # f: 40-41
            return data  # f: 25-47
        except OSError as e:  # f: 42-43
            raise  # f: 25-47
        else:  # f: 44-45
            pass  # f: 25-47
        finally:  # f: 46-47
            pass  # f: 25-47

    def query(self):  # f: 49-75
        sql = """
select *
from t
"""
# a comment at column 0 inside the body
        rows = [  # f: 55-61
            row  # f: 49-75
            for row in self.run(  # f: 57-58
                sql,  # f: 49-75
            ).fetchall()  # f: 49-75
            if row  # f: 49-75
        ]  # f: 49-75
        total = (  # f: 62-68
            len(rows)  # f: 49-75
            if rows  # f: 49-75
            else self.count(  # f: 65-66
                sql,  # f: 49-75
            ).total  # f: 49-75
        )  # f: 49-75
        result = self.post(  # f: 69-70
            rows,  # f: 49-75
        ).json()  # f: 49-75
        return {  # f: 72-75
            "rows": rows,  # f: 49-75
            "total": total,  # f: 49-75
        }  # f: 49-75

    def pick(self, value):  # f: 77-87
        match value:  # f: 78-87
            case (  # f: 79-83
                1  # f: 77-87
                | 2  # f: 77-87
            ):  # f: 77-87
                return "small"  # f: 77-87
            case _:  # f: 84-87
                def inner():  # f: 85-86
                    return "other"  # f: 85-86
                return inner()  # f: 77-87
