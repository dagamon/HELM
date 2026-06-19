import os

HELM_URL: str = os.environ.get("HELM_URL", "http://localhost:7010")
HELM_PIN: str = os.environ.get("HELM_PIN", "")
