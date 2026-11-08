/* A comment nests: /* this one closes inside it */
func huskNested() {}
func huskReal() {}
*/
#if false
func huskDisabled() {}
#elseif DEBUG
func huskDebug() {}
#endif
func huskReal() {}

func huskUse() {
    huskNested()
//  ^ d: none
    huskDisabled()
//  ^ d: none
    huskDebug()
//  ^ d: Sources/Shop/Husk.swift:8
    huskReal()
//  ^ d: Sources/Shop/Husk.swift:10
}
