from requests import post

res = post('http://192.168.0.106:8080/fuckyou',data={"type":"pdf","pages":"0"})

print(res.text)
