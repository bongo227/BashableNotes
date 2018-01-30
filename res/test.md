```dockerfile {"name":"Dockerfile"}
FROM ubuntu:latest
```

```bash {"name":"install.sh", "cmd":"bash install.sh"}
# sudo -H pip install setuptools
# sudo -H pip install matplotlib
# sudo apt-get install -y python-tk
```

```python {"name":"helloworld.py", "cmd":"python helloworld.py"}
print "Hello world!"
```

```python {"name":"bank.py", "cmd":"python bank.py"}
class BankAccount(object):
    def __init__(self, initial_balance=0):
        self.balance = initial_balance
    def deposit(self, amount):
        self.balance += amount
    def withdraw(self, amount):
        self.balance -= amount
    def overdrawn(self):
        return self.balance < 0
my_account = BankAccount(15)
my_account.withdraw(5)
print my_account.balance
```

```python {"name":"graph.py", "cmd":"python graph.py"}
import matplotlib.pyplot as plt
import numpy as np

plt.figure(figsize=[6,6])
x = np.arange(0,100,0.00001)
y = x*np.sin(2*np.pi*x)
plt.plot(y)
plt.axis('off')
plt.gca().set_position([0, 0, 1, 1])
plt.savefig("test.svg")
```

![graph](notebook/test.svg)

```bash {"name":"cmd.sh", "cmd":"bash cmd.sh"}
cat cmd.sh
echo "Hello bash!"
ls -a
```


 Hello world this is some text

 1. What
 2. About
 3. A
 4. List!