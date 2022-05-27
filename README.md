# starry

基于 rust 的插件化工具箱

## 文档

后端默认监听事件:

1. install_extension 安装拓展
2. remove_extension 移除拓展
3. list_all_extension 列出所有拓展

前台默认监听事件:

1. error 遇到错误被调用
2. loaded_extension 拓展安装加载更新或列出拓展被调用
3. unloaded_extension 拓展移除时被调用

## TODO

1. [x] 安装和卸载拓展失败可恢复
2. [ ] 测试插件自动安装拓展的场景
3. [ ] 多插件微前端加载
3. [ ] 更方便的事件发送接收函数
3. [ ] 拓展事件监听根据拓展 id + 事件名称进行识别
3. [ ] 拓展事件发送根据窗口 id 进行识别
2. [ ] 自定义 Event Sender 类型, 避免依赖
4. [ ] 插件和拓展管理界面
5. [ ] 插件开发热重载工具
5. [ ] 插件源服务器