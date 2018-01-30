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

```bash {"name":"cmd.sh", "cmd":"bash cmd.sh"}
echo "Hello bash!"
ls -a
```

 Hello world this is some text

 1. What
 2. About
 3. A
 4. List!