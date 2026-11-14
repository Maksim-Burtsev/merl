package quoted

class Stylus {
    String blot() { 'stylus' }
}

class Nib {
    String blot() { 'nib' }
}

class Quoted {
    String a(Stylus s) { "s.blot() ${s.blot()}" + 's.blot()' }
    //                      ^ d: picker src/main/groovy/quoted/Quoted.groovy:4, src/main/groovy/quoted/Quoted.groovy:8, …
    //                                 ^ d: src/main/groovy/quoted/Quoted.groovy:4
    //                                               ^ d: picker src/main/groovy/quoted/Quoted.groovy:4, src/main/groovy/quoted/Quoted.groovy:8, …
    String b(Stylus s) { "s: $s.blot" + ~/'/ + s.blot() }
    //                            ^ d: src/main/groovy/quoted/Quoted.groovy:4
    //                                              ^ d: src/main/groovy/quoted/Quoted.groovy:4
    String c(Stylus s) {
        """
        \${x//a/b}
        """ + s.blot()
        //      ^ d: src/main/groovy/quoted/Quoted.groovy:4
    }
}
