#include "tea/strainer.hh"
using namespace strainer;

int pour()
{
    finest(1, 2);
//  ^ d: include/tea/strainer.hh:17
    shake(1);
//  ^ d: none
    tilt(1);
//  ^ d: none
    return coarsest(1);
//         ^ d: include/tea/strainer.hh:25
}

double Mesh::sluice() const
{
    return shake(1);
//         ^ d: include/tea/strainer.hh:12
}

template <typename K, typename C>
void Tamis<K, C>::sift(const K& k)
{
    muddle(k);
//  ^ d: include/tea/strainer.hh:38
}

Scoop::operator bool() const
{
    return brim() > 0;
//         ^ d: include/tea/strainer.hh:48
}

void Scoop::tip() __attribute__((noinline))
{
    dredge();
//  ^ d: include/tea/strainer.hh:49
}

Scoop::Scoop() try : level(0)
{
    dredge();
//  ^ d: include/tea/strainer.hh:49
} catch (...) {
}
