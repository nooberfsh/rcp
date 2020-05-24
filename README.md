# rcp
一个类似 `scp` 的工具， 能够通过跳板机直接把文件上传到服务器， 或者从服务器拉取文件到本地。

##  Usage
发送：
```
rcp path/to/local/file remote_addr:path
```
拉取:
```
rcp remote_addr:path/to/remote/file local/path
```

## Config
在 `$HOME` 目录建立 `.rcp` 配置文件, 配置项：
- ip: 跳板机 ip， 必填。
- username: 跳板机用户名， 必填。
- port: 跳板机端口， 选填， 默认`22`。
- private_key: 私钥路径， 选填， 默认 `$HOME/.ssh/id_rsa`。
- scp:跳板机`scp` 程序名， 选填， 默认 `scp`。

## License

MIT