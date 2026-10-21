#include "lab/carafe.h"

int volume = 0;

static int decant(void) { return 2; }

struct carafe *carafe_new(int volume) { return 0; }

struct beaker *beaker_of(struct carafe *c) { return 0; }

int swirl(struct carafe *c) { return c->volume + decant(); }

int swirl_twice(struct carafe *c) { return swirl(c) * 2; }

int burner(int flame) { return flame; }
