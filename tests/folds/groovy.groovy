// Every construct `f` folds in Groovy, and the cases that once broke it.
package demo  // f: none

import groovy.transform.CompileStatic  // f: none

@CompileStatic  // f: none
class Ledger extends Base {  // f: 7-57
    private Map<String, Integer> totals = [  // f: 8-11
        rent: 1200,  // f: 7-57
        food: 300,  // f: 7-57
    ]  // f: 7-57

    Ledger(String owner) {  // f: 13-15
        this.owner = owner  // f: 13-15
    }  // f: 13-15

    def add(String name,  // f: 7-57
            int amount) {  // f: 18-20
        totals[name] = (totals[name] ?: 0) + amount  // f: 18-20
    }  // f: 18-20

    List<String> names() {  // f: 22-29
        def found = totals.keySet().findAll { String k ->  // f: 23-25
            k.size() > 2  // f: 22-29
        }  // f: 22-29
        found.collect {  // f: 26-28
            it.toUpperCase()  // f: 22-29
        }  // f: 22-29
    }  // f: 22-29

    static int code(String kind) {  // f: 31-38
        switch (kind) {  // f: 31-38
            case 'a':  // f: 31-38
                return 1  // f: 31-38
            default:  // f: 31-38
                return 0  // f: 31-38
        }  // f: 31-38
    }  // f: 31-38

    def report() {  // f: 40-56
        if (totals.isEmpty()) {  // f: 41-42
            println 'nothing'  // f: 40-56
        } else {  // f: 43-48
            println """  // f: 44-47
                total: ${totals.values().sum { it }}  // f: 40-56
                } not a closer  // f: 40-56
            """  // f: 40-56
        }  // f: 40-56
        def re = ~/\{[a-z]+/  // f: 40-56
        def path = "${base}/{x}"  // f: 40-56
        String note = '''  // f: 40-56
            { not a closer either  // f: 40-56
        '''  // f: 40-56
        log.info('a {',  // f: 54-55
            'b }')  // f: 40-56
    }  // f: 40-56
}  // f: 7-57

def 'the ledger adds'() {  // f: 59-74
    given:  // f: 59-74
    def l = new Ledger('me')  // f: 59-74
    when:  // f: 59-74
    l.add('rent', 1)  // f: 59-74
    then:  // f: 59-74
    l.totals['rent'] ==  // f: 65-66
        1201  // f: 65-66
    l.names() == [  // f: 67-70
        'RENT',  // f: 67-70
        'FOOD',  // f: 67-70
    ]  // f: 67-70
    renderer.render(l) == '''  // f: 71-73
        rent 1201  // f: 71-73
        '''.stripIndent()  // f: 71-73
}  // f: 59-74

node('linux') {  // f: 76-87
    stage('build') {  // f: 77-86
        sh '''  // f: 78-80
            make all  // f: 78-80
        '''  // f: 78-80
    }  // f: 77-86
    withEnv([  // f: 82-83
        'A=1',  // f: 82-83
    ]) {  // f: 84-86
        sh "make test"  // f: 84-86
    }  // f: 84-86
}  // f: 76-87
