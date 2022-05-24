# starry

基于 rust 的插件化工具箱

## 文档

后端默认监听事件:

1. install_extension
2. remove_extension

前台默认监听事件:

1. error
2. loaded_extension
3. unloaded_extension

## TODO

1. [x] 安装和卸载拓展失败可恢复
2. [ ] 测试插件自动安装拓展的场景
3. [ ] 多插件微前端加载
3. [ ] 拓展事件监听根据拓展 id + 事件名称进行识别
3. [ ] 拓展事件发送根据窗口 id 进行识别
4. [ ] 插件和拓展管理界面
5. [ ] 插件开发热重载工具
5. [ ] 插件源服务器