import requests


class ApiClient:
    def get(self, url, timeout=None):
        return url


class Fetcher:
    def __init__(self):
        self.session = requests.Session()

    def fetch(self, url):
        # `get` is the library's `Session.get`: the project's namesake is only offered.
        return self.session.get(url, timeout=5)
        #                            ^ d: picker shop/client.py:5
