class Route {
  const Route(this.path, {this.auth = false});
  final String path;
  final bool auth;
}
