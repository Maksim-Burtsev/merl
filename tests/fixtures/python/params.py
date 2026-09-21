class RecipeController:
    def get_one(self, slug: str, limit=10) -> str:
        found = slug.strip()
        return found + str(limit)

    def get_many(
        self,
        slugs: list[str],
    ) -> list[str]:
        kept = slugs
        return kept

    async def get_later(self, slug: str) -> str:
        return slug


def top(level: int) -> int:
    return level
