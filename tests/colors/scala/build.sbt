ThisBuild / scalaVersion := "3.5.2"
ThisBuild / organization := "dev.ledger"

lazy val core = (project in file("core"))
  .settings(
    name := "ledger-core",
    libraryDependencies ++= Seq(
      "org.typelevel" %% "cats-core" % "2.12.0",
      "org.scalameta" %% "munit" % "1.0.2" % Test
    ),
    scalacOptions += "-deprecation"
  )
