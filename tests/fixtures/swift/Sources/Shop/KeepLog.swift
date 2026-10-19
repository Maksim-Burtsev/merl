import Kit

func keepLog() {
    Kit.record("x")
//      ^ d: Sources/Kit/Log.swift:1
    Kit.jot("x")
//      ^ d: picker Sources/Kit/Log.swift:3, Sources/Shop/Diary.swift:2
}
