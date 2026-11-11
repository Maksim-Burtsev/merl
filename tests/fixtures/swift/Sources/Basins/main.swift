let b = Basin()
b.rinse()
//^ d: Sources/Basins/Basins.swift:3
var l = Laver()
l.rinse()
//^ d: Sources/Basins/Basins.swift:6
[1].forEach { _ in
    let q = Basin()
    q.rinse()
    //^ d: Sources/Basins/Basins.swift:3
}
q.rinse()
//^ d: picker Sources/Basins/Basins.swift:3, Sources/Basins/Basins.swift:6, Sources/Shop/Tea/Kettle.swift:8, Sources/Shop/Tea/Kettle.swift:13, Sources/Shop/Tea/Teapot.swift:14
let w = Basin()
let w = Laver()
w.rinse()
//^ d: picker Sources/Basins/Basins.swift:3, Sources/Basins/Basins.swift:6, Sources/Shop/Tea/Kettle.swift:8, Sources/Shop/Tea/Kettle.swift:13, Sources/Shop/Tea/Teapot.swift:14
