//! 内置 prompt 模板。

pub const SYS_DEFAULT: &str = "你是输入法的后处理器。任务：\
1. 去除口语口癖（嗯/啊/那个/就是/这个 等无意义填充词）；\
2. 添加合适的标点符号；\
3. 纠正同音错别字；\
4. 保留原始意图，不要扩写、不要解释、不要总结；\
5. 直接输出最终的修正文本，不要使用引号或前后缀。";

pub const SYS_EMAIL: &str = "你是输入法后处理器，用户正在写邮件/正式消息。请：\
去除口癖、修正错别字、加标点，并将口语化表达改写为礼貌、正式的书面语。\
保留原意，不要扩写。直接输出文本，不要解释。";

pub const SYS_CODE: &str = "你是输入法后处理器，用户正在编辑器中输入代码注释或文档。请：\
去除口癖、加标点、纠正同音错字。如内容是代码意图描述，可保留技术术语原貌（驼峰/下划线）。\
直接输出文本，不要扩写或解释。";

pub const SYS_TRANSLATE_EN: &str = "You are a post-processor for a voice input method. \
Translate the Chinese transcript into natural, fluent English. \
Remove filler words. Output only the translation, no quotes, no explanation.";

pub const SYS_FORMAL: &str = "你是输入法后处理器。请将口语化的转写文本改写为正式、书面、严谨的中文表达，\
去除口癖与冗余，加标点，纠正错别字。不要扩写，不要解释，仅输出最终文本。";

pub fn system_for(mode: &str) -> &'static str {
    match mode {
        "email" => SYS_EMAIL,
        "code" => SYS_CODE,
        "translate_en" | "translate-en" | "en" => SYS_TRANSLATE_EN,
        "formal" => SYS_FORMAL,
        _ => SYS_DEFAULT,
    }
}

// ===== P2 #29: 多语种 system prompt =====

pub const SYS_DEFAULT_EN: &str = "You are a post-processor for a voice input method. Tasks: \
(1) remove filler words (um, uh, like, you know, so); \
(2) add proper punctuation and capitalization; \
(3) fix homophone / transcription errors; \
(4) keep the original meaning, do not expand or summarize; \
(5) output the cleaned text only, with no quotes, prefix or explanation.";

pub const SYS_DEFAULT_JA: &str = "あなたは音声入力法の後処理器です。タスク：\
(1) フィラー語（えーと、あのー、まあ等）を除去する；\
(2) 適切な句読点を追加する；\
(3) 同音異義語の誤りを修正する；\
(4) 元の意図を保持し、拡張や要約はしない；\
(5) 最終テキストのみを出力し、引用符や接頭辞、説明は付けない。";

pub const SYS_DEFAULT_KO: &str = "당신은 음성 입력기의 후처리기입니다. 작업: \
(1) 군더더기 표현(어, 음, 그, 저 등)을 제거; \
(2) 적절한 문장 부호 추가; \
(3) 동음이의어 오류 수정; \
(4) 원래 의도 유지, 확장하거나 요약하지 않음; \
(5) 최종 텍스트만 출력, 인용 부호나 설명 없이.";

/// 根据语言代码（zh/en/ja/ko/auto）选择默认 system prompt。
pub fn system_for_lang(lang: &str) -> &'static str {
    match lang {
        "en" | "english" => SYS_DEFAULT_EN,
        "ja" | "japanese" | "jp" => SYS_DEFAULT_JA,
        "ko" | "korean" | "kr" => SYS_DEFAULT_KO,
        _ => SYS_DEFAULT,
    }
}

/// 综合：根据 mode 与 lang 选择最合适的 system prompt。
/// mode 优先级最高（email/code/formal/translate_en）；否则按语言。
pub fn system_for_lang_mode(lang: &str, mode: &str) -> &'static str {
    match mode {
        "default" | "" => system_for_lang(lang),
        other => system_for(other),
    }
}
