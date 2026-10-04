import groovy.transform.Field

@Field String region = 'eu'

void call(Map args) {
    echo region
}
