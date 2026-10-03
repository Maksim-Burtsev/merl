import groovy.transform.Field

def call(Map config = [:]) {
    node {
        sh "mvn -B verify -Dtimeout=${config.timeout ?: 30}"
        //                            ^ d: vars/shipPlugin.groovy:3
    }
}
