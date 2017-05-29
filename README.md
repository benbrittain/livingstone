# livingstone

``` “Tourist, Rincewind decided, meant 'idiot'.” -- Terry Pratchet```

Live at: http://travel.benbrittain.com

Written in Rust for my overland motorcycle Africa trip.

Supports GPX uploads via FTP to be displayed on the map. 

## To Build:
```
cp resources/password.json prod-password.json
vim prod-password.json (edit for your users)
docker build -t livingstone .
```

## To Run (customize port mappings if desired):
```
docker run -it -p 80:8080 -p 2121:2121 livingstone:latest
```
