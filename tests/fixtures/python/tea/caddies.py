from django.db import models


class Named(models.Model):
    name = models.CharField(max_length=10)

    class Meta:
        abstract = True


class Caddy(Named):
    scoops = models.IntegerField()
    blend: str = "green"

    def refill(self):
        self.label = models.CharField()
        brand = models.CharField()
        return brand


def stock(shelf):
    Caddy.objects.create(name="x", scoops=3)
    #                    ^ d: tea/caddies.py:5
    #                              ^ d: tea/caddies.py:12
    Caddy.objects.create(
        scoops=4,
        # ^ d: tea/caddies.py:12
        blend="black",
        # ^ d: none
        label="x",
        # ^ d: none
        brand=2,
        # ^ d: none
        abstract=True,
        # ^ d: none
    )
    Caddy.objects.filter(scoops=3)
    #                    ^ d: none
    shelf.Caddy.objects.create(scoops=3)
    #                          ^ d: none
