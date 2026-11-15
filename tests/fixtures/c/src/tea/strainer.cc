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
