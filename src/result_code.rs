#![allow(dead_code)]
use strum::EnumString;

/// 业务错误码
///
/// 首位编码：
///   0 - 成功
///   A - 用户端错误
///   B - 系统端错误
///   C - 第三方服务错误
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString)]
#[allow(clippy::upper_case_acronyms)]
pub enum ResultCode {
    // 成功
    /// 00000 — 成功
    #[strum(serialize = "00000")]
    Success,

    // A: 用户端错误
    /// A0001 — 用户端错误
    #[strum(serialize = "A0001")]
    UserError,

    // A01xx 用户注册
    #[strum(serialize = "A0100")]
    UserRegistrationError,
    #[strum(serialize = "A0101")]
    UserNotAgreePrivacyAgreement,
    #[strum(serialize = "A0130")]
    VerificationCodeInputError,

    // A02xx 登录异常
    #[strum(serialize = "A0200")]
    UserLoginException,
    #[strum(serialize = "A0201")]
    AccountNotFound,
    #[strum(serialize = "A0202")]
    AccountFrozen,
    #[strum(serialize = "A0210")]
    UserPasswordError,

    // A023x 令牌
    #[strum(serialize = "A0230")]
    AccessTokenInvalid,
    #[strum(serialize = "A0231")]
    RefreshTokenInvalid,

    // A024x 验证码
    #[strum(serialize = "A0240")]
    UserVerificationCodeError,
    #[strum(serialize = "A0241")]
    UserVerificationCodeAttemptLimitExceeded,
    #[strum(serialize = "A0242")]
    UserVerificationCodeExpired,

    // A025x 扫码登录
    #[strum(serialize = "A0250")]
    QrCodeNotFound,
    #[strum(serialize = "A0251")]
    QrCodeStatusIllegal,
    #[strum(serialize = "A0252")]
    QrCodeUserMismatch,
    #[strum(serialize = "A0253")]
    QrCodeAlreadyUsed,

    // A03xx 权限
    #[strum(serialize = "A0300")]
    AccessPermissionException,
    #[strum(serialize = "A0301")]
    AccessUnauthorized,

    // A04xx 参数
    #[strum(serialize = "A0400")]
    UserRequestParameterError,
    #[strum(serialize = "A0402")]
    InvalidUserInput,
    #[strum(serialize = "A0410")]
    RequestRequiredParameterIsEmpty,
    #[strum(serialize = "A0421")]
    ParameterFormatMismatch,

    // A05xx 请求服务
    #[strum(serialize = "A0501")]
    DataNotFound,
    #[strum(serialize = "A0500")]
    UserRequestServiceException,
    #[strum(serialize = "A0502")]
    RequestConcurrencyLimitExceeded,
    #[strum(serialize = "A0506")]
    DuplicateSubmission,

    // A07xx 文件
    #[strum(serialize = "A0700")]
    UploadFileException,
    #[strum(serialize = "A0710")]
    DeleteFileException,

    // B: 系统端错误
    #[strum(serialize = "B0001")]
    SystemError,
    #[strum(serialize = "B0100")]
    SystemExecutionTimeout,

    // C: 第三方服务错误
    #[strum(serialize = "C0001")]
    ThirdPartyServiceError,
    #[strum(serialize = "C0113")]
    InterfaceNotExist,
    #[strum(serialize = "C0300")]
    DatabaseServiceError,
    #[strum(serialize = "C0310")]
    DatabaseExecutionError,
    #[strum(serialize = "C0313")]
    DatabaseExecutionSyntaxError,
    #[strum(serialize = "C0342")]
    IntegrityConstraintViolation,
    #[strum(serialize = "C0351")]
    DatabaseAccessDenied,
}

impl ResultCode {
    /// 获取错误码字符串（如 "A0230"）
    pub fn code(&self) -> &'static str {
        match self {
            Self::Success => "00000",
            Self::UserError => "A0001",
            Self::UserRegistrationError => "A0100",
            Self::UserNotAgreePrivacyAgreement => "A0101",
            Self::VerificationCodeInputError => "A0130",
            Self::UserLoginException => "A0200",
            Self::AccountNotFound => "A0201",
            Self::AccountFrozen => "A0202",
            Self::UserPasswordError => "A0210",
            Self::AccessTokenInvalid => "A0230",
            Self::RefreshTokenInvalid => "A0231",
            Self::UserVerificationCodeError => "A0240",
            Self::UserVerificationCodeAttemptLimitExceeded => "A0241",
            Self::UserVerificationCodeExpired => "A0242",
            Self::QrCodeNotFound => "A0250",
            Self::QrCodeStatusIllegal => "A0251",
            Self::QrCodeUserMismatch => "A0252",
            Self::QrCodeAlreadyUsed => "A0253",
            Self::AccessPermissionException => "A0300",
            Self::AccessUnauthorized => "A0301",
            Self::UserRequestParameterError => "A0400",
            Self::InvalidUserInput => "A0402",
            Self::RequestRequiredParameterIsEmpty => "A0410",
            Self::ParameterFormatMismatch => "A0421",
            Self::DataNotFound => "A0501",
            Self::UserRequestServiceException => "A0500",
            Self::RequestConcurrencyLimitExceeded => "A0502",
            Self::DuplicateSubmission => "A0506",
            Self::UploadFileException => "A0700",
            Self::DeleteFileException => "A0710",
            Self::SystemError => "B0001",
            Self::SystemExecutionTimeout => "B0100",
            Self::ThirdPartyServiceError => "C0001",
            Self::InterfaceNotExist => "C0113",
            Self::DatabaseServiceError => "C0300",
            Self::DatabaseExecutionError => "C0310",
            Self::DatabaseExecutionSyntaxError => "C0313",
            Self::IntegrityConstraintViolation => "C0342",
            Self::DatabaseAccessDenied => "C0351",
        }
    }

    /// 返回中文错误消息
    pub fn msg(&self) -> &'static str {
        match self {
            Self::Success => "成功",
            Self::UserError => "用户端错误",
            Self::UserRegistrationError => "用户注册错误",
            Self::UserNotAgreePrivacyAgreement => "用户未同意隐私协议",
            Self::VerificationCodeInputError => "校验码输入错误",
            Self::UserLoginException => "用户登录异常",
            Self::AccountNotFound => "用户账户不存在",
            Self::AccountFrozen => "用户账户被冻结",
            Self::UserPasswordError => "用户名或密码错误",
            Self::AccessTokenInvalid => "访问令牌无效或已过期",
            Self::RefreshTokenInvalid => "刷新令牌无效或已过期",
            Self::UserVerificationCodeError => "验证码错误",
            Self::UserVerificationCodeAttemptLimitExceeded => "用户验证码尝试次数超限",
            Self::UserVerificationCodeExpired => "用户验证码过期",
            Self::QrCodeNotFound => "扫码登录票据不存在或已过期",
            Self::QrCodeStatusIllegal => "扫码登录状态非法",
            Self::QrCodeUserMismatch => "扫码用户不匹配",
            Self::QrCodeAlreadyUsed => "扫码登录票据已使用",
            Self::AccessPermissionException => "访问权限异常",
            Self::AccessUnauthorized => "访问未授权",
            Self::UserRequestParameterError => "用户请求参数错误",
            Self::InvalidUserInput => "无效的用户输入",
            Self::RequestRequiredParameterIsEmpty => "请求必填参数为空",
            Self::ParameterFormatMismatch => "参数格式不匹配",
            Self::DataNotFound => "数据不存在",
            Self::UserRequestServiceException => "用户请求服务异常",
            Self::RequestConcurrencyLimitExceeded => "请求并发数超出限制",
            Self::DuplicateSubmission => "请勿重复提交",
            Self::UploadFileException => "上传文件异常",
            Self::DeleteFileException => "删除文件异常",
            Self::SystemError => "系统执行出错",
            Self::SystemExecutionTimeout => "系统执行超时",
            Self::ThirdPartyServiceError => "调用第三方服务出错",
            Self::InterfaceNotExist => "接口不存在",
            Self::DatabaseServiceError => "数据库服务出错",
            Self::DatabaseExecutionError => "数据库执行异常",
            Self::DatabaseExecutionSyntaxError => "数据库执行语法错误",
            Self::IntegrityConstraintViolation => "违反了完整性约束",
            Self::DatabaseAccessDenied => "数据库访问被拒绝",
        }
    }
}

// 数据权限枚举

/// 数据权限范围：ALL(1)=全部 / DEPT_AND_SUB(2)=本部门及子部门 / DEPT(3)=本部门 / SELF(4)=本人 / CUSTOM(5)=指定部门
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataScope {
    All = 1,
    DeptAndSub = 2,
    Dept = 3,
    Self_ = 4,
    Custom = 5,
}

impl DataScope {
    pub fn is_all(&self) -> bool {
        matches!(self, DataScope::All)
    }
}

// 状态枚举

/// 通用启用/禁用状态
pub mod status {
    pub const ENABLED: i16 = 1;
    pub const DISABLED: i16 = 0;
}

/// 逻辑删除标识
pub mod deleted {
    pub const NOT_DELETED: i16 = 0;
    pub const DELETED: i16 = 1;
}

/// 菜单类型
pub mod menu_type {
    pub const CATALOG: i16 = 0; // 目录
    pub const MENU: i16 = 1; // 菜单
    pub const BUTTON: i16 = 2; // 按钮
}

/// 性别
pub mod gender {
    pub const MALE: i16 = 1;
    pub const FEMALE: i16 = 2;
    pub const UNKNOWN: i16 = 0;
}

/// 是否（通用）
pub mod yes_no {
    pub const YES: i16 = 1;
    pub const NO: i16 = 0;
}
