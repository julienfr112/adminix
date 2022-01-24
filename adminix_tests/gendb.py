import sqlite3
import os
try:
    os.remove("example.db")
except:
    pass
con = sqlite3.connect('example.db')
cur = con.cursor()
cur.execute('''
    CREATE TABLE stocks 
               (id INTEGER PRIMARY KEY, date TEXT, trans TEXT, symbol TEXT, qty FLOAT, price FLOAT)
''')

cur.execute("INSERT INTO stocks VALUES (1,'2006-01-05','BUY','RHAT',100,35.14)")

con.commit()
con.close()

