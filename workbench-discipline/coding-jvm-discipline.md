---
rule-guard:
  globs:
    - "**/*.{java,kt}"
---
# AI Coding Discipline — Java / Kotlin

> 基于 [luoling8192/ai-coding-principles](https://github.com/luoling8192/ai-coding-principles)，补充 JVM 语言示例。
> 六条规则语言无关，本文件替换 TypeScript 示例为 Kotlin（Java 差异处注明）。

---

## Rule 1: No Silent Fallbacks

**禁止用默认值掩盖不该缺失的数据。**

```kotlin
// FORBIDDEN
val price = product.price ?: BigDecimal.ZERO       // 掩盖契约违反
val name  = user.name  ?: "Unknown"

// CORRECT
val price = requireNotNull(product.price) { "Product ${product.id} missing price" }
// Java: product.getPrice().orElseThrow(() -> new IllegalStateException(...))
```

**允许使用默认值**：UI 占位符、有文档的可选配置、外部输入中缺失本身合法的字段。

写 `?:` / `orElse` 前问：若此处为 null，默认值能产生正确结果吗？不能 → 抛出异常。

---

## Rule 2: No Catch-All in Business Logic

**业务逻辑不包裹整个流程的 try/catch，让异常自然传播。**

```kotlin
// FORBIDDEN
fun createOrder(input: OrderInput): Order? = try {
    val user   = getUser(input.userId)
    val coupon = validateCoupon(input.couponCode)
    saveOrder(input, coupon.value)
} catch (e: Exception) { logger.error(e.message); null }

// CORRECT — 异常向上传播
fun createOrder(input: OrderInput): Order {
    val user   = getUser(input.userId)
    val coupon = validateCoupon(input.couponCode)
    return saveOrder(input, coupon.value)
}

// 只在 Controller / 顶层边界捕获
@PostMapping("/orders")
fun createOrder(@RequestBody input: OrderInput): ResponseEntity<*> = try {
    ResponseEntity.ok(orderService.createOrder(input))
} catch (e: Exception) {
    logger.error("Order creation failed", e)
    ResponseEntity.internalServerError().body(mapOf("error" to "Order creation failed"))
}
```

**永远不要** catch Exception 只是为了 `return null / false / 空对象`。

---

## Rule 3: Tests Must Fail When Code Breaks

**每个测试必须验证具体业务结果，删掉核心逻辑后测试必须失败。**

```kotlin
// FORBIDDEN — 实现返回垃圾仍然通过
@Test fun `should process order`() {
    val result = orderService.processOrder(mockOrder)
    assertNotNull(result)          // 只验证非 null，毫无意义
}

// CORRECT — 验证精确的业务结果（Kotest / AssertJ 均可）
@Test fun `should calculate total with 10% discount`() {
    val result = orderService.processOrder(mockOrder.copy(discount = 0.1))
    result.totalAmount    shouldBe BigDecimal("900.00")
    result.discountAmount shouldBe BigDecimal("100.00")
    result.status         shouldBe OrderStatus.CONFIRMED
}
// Java: assertThat(result.getTotalAmount()).isEqualByComparingTo("900.00");
```

**禁止作为唯一断言**：`assertNotNull` / `assertTrue(x != null)` / `isNotEmpty()` 单独使用。

---

## Rule 4: No Hardcoded Lookup-Table Implementations

**禁止用硬编码返回值蒙测试，实现真实逻辑。**

```kotlin
// FORBIDDEN
fun calculateDiscount(amount: BigDecimal, level: String) = when {
    amount == BigDecimal("1000") && level == "gold"   -> BigDecimal("100")
    amount == BigDecimal("500")  && level == "silver" -> BigDecimal("25")
    else -> BigDecimal.ZERO
}

// CORRECT
fun calculateDiscount(amount: BigDecimal, level: String): BigDecimal {
    val rates = mapOf("gold" to 0.10, "silver" to 0.05, "bronze" to 0.02)
    return amount * BigDecimal(rates[level] ?: 0.0)
}
```

防止手段：用 `@ParameterizedTest` / Kotest `forAll` 加入原始规格之外的边界值。

---

## Rule 5: Red-Green Testing (TDD Order)

**修复 bug 时，先写失败测试，再修代码。**

```
1. 发现 bug
2. 写出复现 bug 的测试（JUnit 5 / Kotest）
3. 运行 → 确认 FAIL（红）
4. 修复代码
5. 运行 → 确认 PASS（绿）
```

**绝不跳过第 3 步**。只有亲眼看到红 → 绿，才能证明这个测试有效。

---

## Rule 6: Never Remove Debug Logs During a Fix

**调试日志只能在人工确认修复有效后才删除。**

```
FORBIDDEN: 找到问题 → 在同一次改动中既修复代码又删除 log.debug / println
CORRECT:   找到问题 → 只修复代码 → 人工确认 → 人工决定删除日志
```

日志删除与代码修复是两个独立的改动，不合并。

---

## 快速自查

| 检查项 | 问题 |
|--------|------|
| 静默回退 | 用 `?:` / `orElse` 隐藏了不该缺失的值？ |
| 异常处理 | 在业务逻辑里 catch-all 阻断了异常传播？ |
| 测试强度 | 删掉实现后测试还会通过？ |
| 测试诚实 | 用硬编码值匹配测试而非实现真实逻辑？ |
| TDD 顺序 | 修 bug 前看到过红测试了吗？ |
| 调试日志 | 在同一改动里既修复代码又删除了诊断日志？ |
