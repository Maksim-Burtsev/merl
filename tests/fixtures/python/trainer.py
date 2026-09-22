class Model:
    def to(self, device) -> "Model":
        return self

    def forward(self, x):
        return x


class Trainer:
    def __init__(self, model: Model, device):
        self.model = model
        self.model = self.model.to(device)

    def step(self, x):
        return self.model.forward(x)
