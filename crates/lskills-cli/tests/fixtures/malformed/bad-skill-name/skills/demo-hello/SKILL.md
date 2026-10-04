---
name: Demo_Hello
description: The name field violates the lowercase-hyphen newtype contract.
---

# Hello

`name: Demo_Hello` has uppercase and an underscore, so `SkillFullName::parse` rejects it.
