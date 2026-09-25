<div align="center">

# <img alt="youlai-axum" width="28" src="./docs/images/logo/logo.png" valign="middle"> youlai-axum



**Rust 企业级权限管理系统后端（Axum + SeaORM）**

[![Rust](https://img.shields.io/badge/Rust-1.82+-000000?logo=rust)](https://www.rust-lang.org/)
[![Axum](https://img.shields.io/badge/Axum-0.8-4B8BBE)](https://github.com/tokio-rs/axum)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-16+-4169E1?logo=postgresql&logoColor=white)](https://www.postgresql.org/)
[![Redis](https://img.shields.io/badge/Redis-7.x-DC382D?logo=redis&logoColor=white)](https://redis.io/)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue?logo=apache)](LICENSE)
[![Gitee Star](https://gitee.com/youlaiorg/youlai-axum/badge/star.svg)](https://gitee.com/youlaiorg/youlai-axum/stargazers)
[![GitHub Star](https://img.shields.io/github/stars/youlaitech/youlai-axum?style=social)](https://github.com/youlaitech/youlai-axum)

</div>

![](./docs/images/rainbow.png "rainbow.png")

<div align="center">

[![在线预览](https://img.shields.io/badge/在线预览-2D8CF0?style=for-the-badge&logo=google-chrome&logoColor=white)](https://vue.youlai.tech)
[![移动端预览](https://img.shields.io/badge/移动端预览-19BE6B?style=for-the-badge&logo=android&logoColor=white)](https://app.youlai.tech)
[![阅读文档](https://img.shields.io/badge/阅读文档-8B5CF6?style=for-the-badge&logo=gitbook&logoColor=white)](https://www.youlai.tech/docs/server/spring-boot/)
[![English](https://img.shields.io/badge/English-00B4D8?style=for-the-badge&logo=google-translate&logoColor=white)](./README.en.md)

</div>

## 项目简介

**youlai-axum** 是一套基于 Rust（Axum + SeaORM + PostgreSQL）的企业级权限管理系统后端，配套前端 [vue3-element-admin](https://gitee.com/youlaiorg/vue3-element-admin) 和移动端 [youlai-app](https://gitee.com/youlaiorg/youlai-app)，并提供 **7 种语言实现**（Java / Node.js / Go / Python / PHP / C# / Rust），共享同一套 API 规范与数据库结构。适用于追求高并发、低延迟的 Rust 技术栈团队学习与二次开发。

## 核心特性

- 🔐 **双模式认证** — JWT 自包含 / Redis Token 有状态，可切换，支持令牌续期
- 🛡️ **细粒度权限** — RBAC 数据 / 菜单 / 按钮 / 接口级，数据权限五档
- ⚡ **代码生成器** — 一键生成前后端 CRUD 代码
- 📦 **模块齐全** — 用户、角色、菜单、部门、字典、配置、文件、通知、操作日志
- 🌐 **多语言生态** — 与其它语言版本共享 API 规范与数据库结构
- 🔌 **实时通信** — SSE 推送：在线用户数、字典同步、通知广播

## 系统预览

**PC 端**

<table align="center">
  <tr>
    <td><img alt="PC预览1" width="400" src="./docs/images/preview/pc-01.png"></td>
    <td><img alt="PC预览2" width="400" src="./docs/images/preview/pc-02.png"></td>
  </tr>
  <tr>
    <td><img alt="PC预览3" width="400" src="./docs/images/preview/pc-03.png"></td>
    <td><img alt="PC预览4" width="400" src="./docs/images/preview/pc-04.png"></td>
  </tr>
  <tr>
    <td><img alt="PC预览5" width="400" src="./docs/images/preview/pc-05.png"></td>
    <td><img alt="PC预览6" width="400" src="./docs/images/preview/pc-06.png"></td>
  </tr>
</table>

**移动端**

<table align="center">
  <tr>
    <td><img alt="APP预览1" width="200" src="./docs/images/preview/app-01.png"></td>
    <td><img alt="APP预览2" width="200" src="./docs/images/preview/app-02.png"></td>
    <td><img alt="APP预览3" width="200" src="./docs/images/preview/app-03.png"></td>
    <td><img alt="APP预览4" width="200" src="./docs/images/preview/app-04.png"></td>
  </tr>
</table>

## 快速开始

**环境要求**：Rust 1.82+ · PostgreSQL 16+ · Redis 7.x · CMake + VS Build Tools（Windows 必装）

> **Windows 用户**：Rust 默认使用 MSVC 工具链，需要 Visual Studio Build Tools 中的 C++ 工作负载。下载 [vs_buildtools.exe](https://aka.ms/vs/17/release/vs_buildtools.exe)，安装时勾选「使用 C++ 的桌面开发」，否则会遇到 `linker link.exe not found`。

1. 克隆项目：`git clone https://github.com/youlaitech/youlai-axum.git`
2. 导入数据库：`psql -U postgres -d youlai_admin -f sql/youlai_admin.sql`
3. 配置环境：`cp .env.example .env`
4. 启动服务：

   **方式一：RustRover 启动（推荐）**
   用 RustRover 打开项目，等待索引完成后点击 `cargo run` 运行配置即可。

   **方式二：命令行启动**
   ```bash
   cargo run
   ```
   启动后访问 [http://localhost:8000/swagger-ui](http://localhost:8000/swagger-ui)，能打开接口文档即说明后端已正常运行。

5. 启动前端（可选）：
   如需可视化操作界面，启动配套前端 [vue3-element-admin](https://gitee.com/youlaiorg/vue3-element-admin)，访问 [http://localhost:3000](http://localhost:3000)，使用 `admin` / `123456` 登录。

**Docker 部署**：`cd docker`，然后 `docker-compose up -d`

## 目录结构

```
youlai-axum/
├── config/                  # 配置文件
│   ├── config.toml          # 主配置（兜底默认值）
│   └── config-dev.toml      # 开发环境覆盖
├── docker/                  # Docker 部署
├── src/
│   ├── main.rs              # 程序入口
│   ├── lib.rs               # 库入口
│   ├── common/              # 公共常量 / 枚举 / 分页
│   ├── core/                # 数据库 / Redis / 配置
│   ├── entity/              # SeaORM Entity（16 张表）
│   ├── framework/           # 框架层
│   │   ├── security/        # 认证 / 授权 / 数据权限
│   │   ├── middleware/      # CORS / 日志 / 限流
│   │   ├── web/             # 统一响应 / 异常处理
│   │   ├── sse/             # SSE 实时推送
│   │   ├── captcha/         # 图形验证码
│   │   └── apidoc.rs        # OpenAPI 文档
│   └── modules/             # 业务模块
│       ├── auth/            # 登录 / 登出 / 刷新令牌
│       ├── system/          # 用户 / 角色 / 菜单 / 部门 / 字典 / 配置 / 通知 / 日志
│       ├── message/         # SSE 消息推送
│       ├── file/            # 文件管理
│       └── codegen/         # 代码生成
├── .env                     # 环境变量（本地开发）
├── .env.example             # 环境变量模板
├── Cargo.toml               # 依赖管理
└── rust-toolchain.toml      # Rust 工具链
```

## 生态矩阵

**前端**

| 项目 | 技术栈 | 说明 | 更新状态 |
|:-----|:-------|:-----|:---------|
| [vue3-element-admin](https://gitee.com/youlaiorg/vue3-element-admin) | Vue 3 + Element Plus | PC 管理前端（主推） | ✅️ |
| [youlai-app](https://gitee.com/youlaiorg/youlai-app) | Vue 3 + UniApp | 移动端 App | ✅️ |

**后端**

| 项目 | 技术栈 | 说明 | 更新状态 |
|:-----|:-------|:-----|:---------|
| [youlai-boot](https://gitee.com/youlaiorg/youlai-boot) | Spring Boot + MyBatis-Plus | Java（主推） | ✅️ |
| [youlai-nest](https://gitee.com/youlaiorg/youlai-nest) | NestJS + TypeORM | Node.js | ✅️ |
| [youlai-gin](https://gitee.com/youlaiorg/youlai-gin) | Go + Gorm | Go | ✅️ |
| [youlai-django](https://gitee.com/youlaiorg/youlai-django) | Django + DRF | Python | ✅️ |
| [youlai-fastapi](https://gitee.com/youlaiorg/youlai-fastapi) | FastAPI + SQLAlchemy | Python | ✅️ |
| [youlai-think](https://gitee.com/youlaiorg/youlai-think) | ThinkPHP + ThinkORM | PHP | ✅️ |
| [youlai-aspnet](https://gitee.com/youlaiorg/youlai-aspnet) | ASP.NET Core + EF Core | C# | ✅️ |
| [youlai-axum](https://gitee.com/youlaiorg/youlai-axum) | Axum + SeaORM | Rust | ✅️ |
| [youlai-laravel](https://gitee.com/youlaiorg/youlai-laravel) | Laravel + Eloquent | PHP | ❌️ |

> 九种后端共享同一套 **RESTful API 规范** 和 **数据库结构**，前端可无缝切换。

**衍生版本**

| 项目 | 基于 | 类型 | 说明 | 更新状态 |
|:-----|:-----|:-----|:-----|:---------|
| [youlai-boot-tenant](https://gitee.com/youlaiorg/youlai-boot-tenant) | youlai-boot | 独立仓库 | 多租户 SaaS，租户隔离与租户配置 | ✅️ |
| [youlai-boot-flex](https://gitee.com/youlaiorg/youlai-boot-flex) | youlai-boot | 独立仓库 | 改用 MyBatis-Flex | ✅️ |
| [youlai-boot (db-pg)](https://gitee.com/youlaiorg/youlai-boot/tree/db-pg) | youlai-boot | 分支 | PostgreSQL 数据库分支 | ✅️ |
| [youlai-boot (multi-module)](https://gitee.com/youlaiorg/youlai-boot/tree/multi-module) | youlai-boot | 分支 | 多模块工程拆分 | ✅️ |
| [youlai-nest (multi-tenant)](https://gitee.com/youlaiorg/youlai-nest/tree/multi-tenant) | youlai-nest | 分支 | 多租户 SaaS，租户隔离与租户配置 | ✅️ |
| [youlai-boot (spring-boot-3)](https://gitee.com/youlaiorg/youlai-boot/tree/spring-boot-3) | youlai-boot | 分支 | Spring Boot 3 兼容分支 | ❌️ |

## 技术合作

本项目采用 [Apache License 2.0](LICENSE) 开源，可免费商用。欢迎在 [Issue](https://gitee.com/youlaiorg/youlai-axum/issues) 提交问题或反馈，也欢迎提交 [Pull Request](https://gitee.com/youlaiorg/youlai-axum/pulls) 共建。

如需技术支持、商务合作、二次开发、项目定制或私有化部署，可联系作者微信（见下方二维码）。

<table align="center">
  <tr>
    <td align="center">
      <img src="./docs/images/qrcode/wechat-official.jpg" height="180" alt="公众号「有来技术」"><br>
      <sub>公众号「有来技术」</sub>
    </td>
    <td>&nbsp;&nbsp;&nbsp;&nbsp;</td>
    <td align="center">
      <img src="./docs/images/qrcode/wechat-mp.jpg" height="180" alt="小程序「有来技术」"><br>
      <sub>小程序「有来技术」</sub>
    </td>
    <td>&nbsp;&nbsp;&nbsp;&nbsp;</td>
    <td align="center">
      <img src="./docs/images/qrcode/wechat-personal.png" height="180" alt="添加作者微信"><br>
      <sub>添加作者微信</sub>
    </td>
  </tr>
</table>

<p align="center"><em>技术交流 · 问题反馈 · 商务合作</em></p>
