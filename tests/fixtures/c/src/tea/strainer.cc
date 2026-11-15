#include "tea/strainer.hh"
using namespace strainer;

int pour()
{
    finest(1, 2);
//  ^ d: include/tea/strainer.hh:14
    return coarsest(1);
//         ^ d: include/tea/strainer.hh:22
}
