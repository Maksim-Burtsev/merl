from relay1 import Parcel
from relay0 import Parcel as Far


def near(parcel: Parcel) -> None:
    parcel.wrap_up()


def far(bundle: Far) -> None:
    bundle.wrap_up()
