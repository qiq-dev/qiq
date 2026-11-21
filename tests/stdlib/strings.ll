@.fmt.int = private unnamed_addr constant [6 x i8] c"%lld\0A\00", align 1
@.fmt.bool = private unnamed_addr constant [4 x i8] c"%d\0A\00", align 1
@.fmt.double = private unnamed_addr constant [4 x i8] c"%g\0A\00", align 1
@.fmt.string = private unnamed_addr constant [4 x i8] c"%s\0A\00", align 1
@.str.null = private unnamed_addr constant [5 x i8] c"null\00", align 1
@.str0 = private unnamed_addr constant [14 x i8] c"h\C3\A9llo w\C3\B6rld\00", align 1
@.str1 = private unnamed_addr constant [7 x i8] c"w\C3\B6rld\00", align 1
@.str2 = private unnamed_addr constant [8 x i8] c"missing\00", align 1
@.str3 = private unnamed_addr constant [3 x i8] c"\C3\B6\00", align 1
@.str4 = private unnamed_addr constant [4 x i8] c"h\C3\A9\00", align 1
@.str5 = private unnamed_addr constant [4 x i8] c"w\C3\B6\00", align 1
@.str6 = private unnamed_addr constant [6 x i8] c"\C3\B6rld\00", align 1
@.str7 = private unnamed_addr constant [3 x i8] c"\C3\B6\00", align 1
@.str8 = private unnamed_addr constant [3 x i8] c"oe\00", align 1
@.str9 = private unnamed_addr constant [3 x i8] c"ab\00", align 1
@.str10 = private unnamed_addr constant [25 x i8] c"\C3\80\C3\89\C3\8E\C3\95\C3\9C \CE\91\CE\92\CE\93 \D0\90\D0\91\D0\92\00", align 1
@.str11 = private unnamed_addr constant [6 x i8] c"\C3\A9lan\00", align 1
@.str12 = private unnamed_addr constant [8 x i8] c"a\C3\B1b\E7\95\8C\00", align 1
@.str13 = private unnamed_addr constant [13 x i8] c"  \09 padded \0A\00", align 1
@.str14 = private unnamed_addr constant [5 x i8] c" \09\0A\0D\00", align 1
@.str15 = private unnamed_addr constant [6 x i8] c"hello\00", align 1
@.str16 = private unnamed_addr constant [3 x i8] c", \00", align 1
@.str17 = private unnamed_addr constant [7 x i8] c"w\C3\B6rld\00", align 1
@.str18 = private unnamed_addr constant [2 x i8] c"!\00", align 1
@.str19 = private unnamed_addr constant [3 x i8] c"\C3\A9\00", align 1
@.str20 = private unnamed_addr constant [3 x i8] c"\C3\BC\00", align 1
@.str21 = private unnamed_addr constant [6 x i8] c"ab\E7\95\8C\00", align 1
@.str22 = private unnamed_addr constant [2 x i8] c",\00", align 1
@.str23 = private unnamed_addr constant [6 x i8] c"a,b,c\00", align 1
@.str24 = private unnamed_addr constant [4 x i8] c" + \00", align 1
@.str25 = private unnamed_addr constant [2 x i8] c"|\00", align 1
@.str26 = private unnamed_addr constant [2 x i8] c",\00", align 1
@.str27 = private unnamed_addr constant [8 x i8] c"a,b,c,d\00", align 1
@.class.name0 = private unnamed_addr constant [10 x i8] c"Exception\00", align 1
@.class.name1 = private unnamed_addr constant [17 x i8] c"RuntimeException\00", align 1
@.class.name2 = private unnamed_addr constant [12 x i8] c"IOException\00", align 1
@.class.name3 = private unnamed_addr constant [22 x i8] c"FileNotFoundException\00", align 1
@.class.name4 = private unnamed_addr constant [26 x i8] c"PermissionDeniedException\00", align 1
@.class.name5 = private unnamed_addr constant [21 x i8] c"KeyNotFoundException\00", align 1
@.class.name6 = private unnamed_addr constant [20 x i8] c"OutOfRangeException\00", align 1
@.class.name7 = private unnamed_addr constant [21 x i8] c"InvalidUtf8Exception\00", align 1
@.class.name8 = private unnamed_addr constant [20 x i8] c"ArithmeticException\00", align 1
@.class.name9 = private unnamed_addr constant [24 x i8] c"DivisionByZeroException\00", align 1
@.class.name10 = private unnamed_addr constant [25 x i8] c"InvalidArgumentException\00", align 1
@qiq_class_names = unnamed_addr constant [11 x ptr] [ptr @.class.name0, ptr @.class.name1, ptr @.class.name2, ptr @.class.name3, ptr @.class.name4, ptr @.class.name5, ptr @.class.name6, ptr @.class.name7, ptr @.class.name8, ptr @.class.name9, ptr @.class.name10], align 8
@qiq_pending_exception = external global ptr

declare i32 @printf(ptr, ...)
declare i32 @strcmp(ptr, ptr)
declare ptr @calloc(i64, i64)
declare ptr @qiq_map_new(i32)
declare void @qiq_map_set(ptr, i64, i64)
declare i64 @qiq_map_get(ptr, i64)
declare i64 @qiq_map_len(ptr)
declare i64 @qiq_map_key_at(ptr, i64)
declare i64 @qiq_map_value_at(ptr, i64)
declare i64 @qiq_string_len(ptr)
declare ptr @qiq_string_char_at(ptr, i64)
declare ptr @qiq_string_concat(ptr, ptr)
declare i64 @qiq_int_divide(i64, i64)
declare i64 @qiq_int_modulo(i64, i64)
declare i64 @qiq_int_power(i64, i64)
declare i64 @qiq_int_shift_left(i64, i64)
declare i64 @qiq_int_shift_right(i64, i64)
declare double @llvm.pow.f64(double, double)
declare void @qiq_uncaught(ptr, ptr)
declare ptr @qiq_array_new(i64, i64)
declare i64 @qiq_array_length(ptr)
declare void @qiq_array_check_index(ptr, i64)
declare void @qiq_print_value(ptr, i64)
declare void @qiq_print_space()
declare void @qiq_print_newline()
declare i64 @qiq_double_to_int(double)
declare zeroext i8 @qiq_int_to_byte(i64)
declare i64 @qiq_string_to_int(ptr)
declare double @qiq_string_to_double(ptr)
declare ptr @qiq_int_to_string(i64)
declare ptr @qiq_double_to_string(double)
declare i64 @qiq_stdlib_strlen(ptr)
declare ptr @qiq_stdlib_substr(ptr, i64, i64)
declare i64 @qiq_stdlib_strpos(ptr, ptr, i64)
declare zeroext i1 @qiq_stdlib_str_contains(ptr, ptr)
declare zeroext i1 @qiq_stdlib_str_ends_with(ptr, ptr)
declare ptr @qiq_stdlib_str_replace(ptr, ptr, ptr)
declare ptr @qiq_stdlib_str_repeat(ptr, i64)
declare ptr @qiq_stdlib_strtoupper(ptr)
declare ptr @qiq_stdlib_strtolower(ptr)
declare ptr @qiq_stdlib_ucfirst(ptr)
declare ptr @qiq_stdlib_strrev(ptr)
declare ptr @qiq_stdlib_trim(ptr, ptr)
declare ptr @qiq_stdlib_explode(ptr, ptr, i64)
declare ptr @qiq_stdlib_implode(ptr, ptr)

define void @qiq_stdlib.exceptions.Exception___construct(ptr %this, ptr %arg0, i64 %arg1) {
entry:
  %v0 = alloca ptr, align 8
  %v1 = alloca i64, align 8
  store ptr %arg0, ptr %v0, align 8
  store i64 %arg1, ptr %v1, align 8
  %v2 = load ptr, ptr %v0, align 8
  %v3 = ptrtoint ptr %v2 to i64
  %v4 = getelementptr inbounds i64, ptr %this, i64 1
  store i64 %v3, ptr %v4, align 8
  %v5 = load i64, ptr %v1, align 8
  %v6 = add i64 %v5, 0
  %v7 = getelementptr inbounds i64, ptr %this, i64 2
  store i64 %v6, ptr %v7, align 8
  ret void
unwind:
  ret void
}

define ptr @qiq_stdlib.exceptions.Exception_getMessage(ptr %this) {
entry:
  %v0 = getelementptr inbounds i64, ptr %this, i64 1
  %v1 = load i64, ptr %v0, align 8
  %v2 = inttoptr i64 %v1 to ptr
  ret ptr %v2
unwind:
  ret ptr null
}

define i64 @qiq_stdlib.exceptions.Exception_getCode(ptr %this) {
entry:
  %v0 = getelementptr inbounds i64, ptr %this, i64 2
  %v1 = load i64, ptr %v0, align 8
  %v2 = add i64 %v1, 0
  ret i64 %v2
unwind:
  ret i64 0
}

define i1 @qiq.stdlib.string.str_starts_with(ptr %arg0, ptr %arg1) {
entry:
  %v0 = alloca ptr, align 8
  %v1 = alloca ptr, align 8
  store ptr %arg0, ptr %v0, align 8
  store ptr %arg1, ptr %v1, align 8
  %v2 = load ptr, ptr %v0, align 8
  %v3 = load ptr, ptr %v1, align 8
  %v4 = call i64 @qiq_stdlib_strlen(ptr %v3)
  %pending0 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown0 = icmp ne ptr %pending0, null
  br i1 %thrown0, label %unwind, label %ok0
ok0:
  %v5 = call ptr @qiq_stdlib_substr(ptr %v2, i64 0, i64 %v4)
  %pending1 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown1 = icmp ne ptr %pending1, null
  br i1 %thrown1, label %unwind, label %ok1
ok1:
  %v6 = load ptr, ptr %v1, align 8
  %v7 = call i32 @strcmp(ptr %v5, ptr %v6)
  %v8 = icmp eq i32 %v7, 0
  br i1 %v8, label %if.then2, label %if.else2
if.then2:
  ret i1 true
if.else2:
  br label %if.end2
if.end2:
  ret i1 false
unwind:
  ret i1 false
}

define void @qiq_raise(i32 %kind, ptr %message) {
entry:
  switch i32 %kind, label %kind0 [ i32 1, label %kind1 i32 2, label %kind2 i32 3, label %kind3 i32 4, label %kind4 i32 5, label %kind5 i32 6, label %kind6 i32 7, label %kind7 i32 8, label %kind8 i32 9, label %kind9 ]
kind0:
  %object0 = call ptr @calloc(i64 3, i64 8)
  store i64 1, ptr %object0, align 8
  call void @qiq_stdlib.exceptions.Exception___construct(ptr %object0, ptr %message, i64 0)
  store ptr %object0, ptr @qiq_pending_exception, align 8
  ret void
kind1:
  %object1 = call ptr @calloc(i64 3, i64 8)
  store i64 8, ptr %object1, align 8
  call void @qiq_stdlib.exceptions.Exception___construct(ptr %object1, ptr %message, i64 0)
  store ptr %object1, ptr @qiq_pending_exception, align 8
  ret void
kind2:
  %object2 = call ptr @calloc(i64 3, i64 8)
  store i64 9, ptr %object2, align 8
  call void @qiq_stdlib.exceptions.Exception___construct(ptr %object2, ptr %message, i64 0)
  store ptr %object2, ptr @qiq_pending_exception, align 8
  ret void
kind3:
  %object3 = call ptr @calloc(i64 3, i64 8)
  store i64 5, ptr %object3, align 8
  call void @qiq_stdlib.exceptions.Exception___construct(ptr %object3, ptr %message, i64 0)
  store ptr %object3, ptr @qiq_pending_exception, align 8
  ret void
kind4:
  %object4 = call ptr @calloc(i64 3, i64 8)
  store i64 7, ptr %object4, align 8
  call void @qiq_stdlib.exceptions.Exception___construct(ptr %object4, ptr %message, i64 0)
  store ptr %object4, ptr @qiq_pending_exception, align 8
  ret void
kind5:
  %object5 = call ptr @calloc(i64 3, i64 8)
  store i64 10, ptr %object5, align 8
  call void @qiq_stdlib.exceptions.Exception___construct(ptr %object5, ptr %message, i64 0)
  store ptr %object5, ptr @qiq_pending_exception, align 8
  ret void
kind6:
  %object6 = call ptr @calloc(i64 3, i64 8)
  store i64 2, ptr %object6, align 8
  call void @qiq_stdlib.exceptions.Exception___construct(ptr %object6, ptr %message, i64 0)
  store ptr %object6, ptr @qiq_pending_exception, align 8
  ret void
kind7:
  %object7 = call ptr @calloc(i64 3, i64 8)
  store i64 3, ptr %object7, align 8
  call void @qiq_stdlib.exceptions.Exception___construct(ptr %object7, ptr %message, i64 0)
  store ptr %object7, ptr @qiq_pending_exception, align 8
  ret void
kind8:
  %object8 = call ptr @calloc(i64 3, i64 8)
  store i64 4, ptr %object8, align 8
  call void @qiq_stdlib.exceptions.Exception___construct(ptr %object8, ptr %message, i64 0)
  store ptr %object8, ptr @qiq_pending_exception, align 8
  ret void
kind9:
  %object9 = call ptr @calloc(i64 3, i64 8)
  store i64 6, ptr %object9, align 8
  call void @qiq_stdlib.exceptions.Exception___construct(ptr %object9, ptr %message, i64 0)
  store ptr %object9, ptr @qiq_pending_exception, align 8
  ret void
}
define i32 @main() {
entry:
  %v0 = alloca ptr, align 8
  %v34 = alloca ptr, align 8
  %v35 = alloca ptr, align 8
  %foreach.slot24 = alloca i64, align 8
  %v52 = alloca ptr, align 8
  store ptr getelementptr inbounds ([14 x i8], ptr @.str0, i32 0, i32 0), ptr %v0, align 8
  %v1 = load ptr, ptr %v0, align 8
  %v2 = call i64 @qiq_stdlib_strlen(ptr %v1)
  %pending0 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown0 = icmp ne ptr %pending0, null
  br i1 %thrown0, label %unwind, label %ok0
ok0:
  call i32 (ptr, ...) @printf(ptr @.fmt.int, i64 %v2)
  %v3 = load ptr, ptr %v0, align 8
  %v4 = call ptr @qiq_stdlib_substr(ptr %v3, i64 1, i64 4)
  %pending1 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown1 = icmp ne ptr %pending1, null
  br i1 %thrown1, label %unwind, label %ok1
ok1:
  call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr %v4)
  %v5 = load ptr, ptr %v0, align 8
  %v6 = call ptr @qiq_stdlib_substr(ptr %v5, i64 6, i64 100)
  %pending2 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown2 = icmp ne ptr %pending2, null
  br i1 %thrown2, label %unwind, label %ok2
ok2:
  call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr %v6)
  %v7 = load ptr, ptr %v0, align 8
  %v8 = call i64 @qiq_stdlib_strpos(ptr %v7, ptr getelementptr inbounds ([7 x i8], ptr @.str1, i32 0, i32 0), i64 0)
  %pending3 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown3 = icmp ne ptr %pending3, null
  br i1 %thrown3, label %unwind, label %ok3
ok3:
  call i32 (ptr, ...) @printf(ptr @.fmt.int, i64 %v8)
  %v9 = load ptr, ptr %v0, align 8
  %v10 = call i64 @qiq_stdlib_strpos(ptr %v9, ptr getelementptr inbounds ([8 x i8], ptr @.str2, i32 0, i32 0), i64 0)
  %pending4 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown4 = icmp ne ptr %pending4, null
  br i1 %thrown4, label %unwind, label %ok4
ok4:
  call i32 (ptr, ...) @printf(ptr @.fmt.int, i64 %v10)
  %v11 = load ptr, ptr %v0, align 8
  %v12 = call i1 @qiq_stdlib_str_contains(ptr %v11, ptr getelementptr inbounds ([3 x i8], ptr @.str3, i32 0, i32 0))
  %pending5 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown5 = icmp ne ptr %pending5, null
  br i1 %thrown5, label %unwind, label %ok5
ok5:
  %v13 = zext i1 %v12 to i32
  call i32 (ptr, ...) @printf(ptr @.fmt.bool, i32 %v13)
  %v14 = load ptr, ptr %v0, align 8
  %v15 = call i1 @qiq.stdlib.string.str_starts_with(ptr %v14, ptr getelementptr inbounds ([4 x i8], ptr @.str4, i32 0, i32 0))
  %pending6 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown6 = icmp ne ptr %pending6, null
  br i1 %thrown6, label %unwind, label %ok6
ok6:
  %v16 = zext i1 %v15 to i32
  call i32 (ptr, ...) @printf(ptr @.fmt.bool, i32 %v16)
  %v17 = load ptr, ptr %v0, align 8
  %v18 = call i1 @qiq.stdlib.string.str_starts_with(ptr %v17, ptr getelementptr inbounds ([4 x i8], ptr @.str5, i32 0, i32 0))
  %pending7 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown7 = icmp ne ptr %pending7, null
  br i1 %thrown7, label %unwind, label %ok7
ok7:
  %v19 = zext i1 %v18 to i32
  call i32 (ptr, ...) @printf(ptr @.fmt.bool, i32 %v19)
  %v20 = load ptr, ptr %v0, align 8
  %v21 = call i1 @qiq_stdlib_str_ends_with(ptr %v20, ptr getelementptr inbounds ([6 x i8], ptr @.str6, i32 0, i32 0))
  %pending8 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown8 = icmp ne ptr %pending8, null
  br i1 %thrown8, label %unwind, label %ok8
ok8:
  %v22 = zext i1 %v21 to i32
  call i32 (ptr, ...) @printf(ptr @.fmt.bool, i32 %v22)
  %v23 = load ptr, ptr %v0, align 8
  %v24 = call ptr @qiq_stdlib_str_replace(ptr getelementptr inbounds ([3 x i8], ptr @.str7, i32 0, i32 0), ptr getelementptr inbounds ([3 x i8], ptr @.str8, i32 0, i32 0), ptr %v23)
  %pending9 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown9 = icmp ne ptr %pending9, null
  br i1 %thrown9, label %unwind, label %ok9
ok9:
  call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr %v24)
  %v25 = call ptr @qiq_stdlib_str_repeat(ptr getelementptr inbounds ([3 x i8], ptr @.str9, i32 0, i32 0), i64 3)
  %pending10 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown10 = icmp ne ptr %pending10, null
  br i1 %thrown10, label %unwind, label %ok10
ok10:
  call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr %v25)
  %v26 = load ptr, ptr %v0, align 8
  %v27 = call ptr @qiq_stdlib_strtoupper(ptr %v26)
  %pending11 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown11 = icmp ne ptr %pending11, null
  br i1 %thrown11, label %unwind, label %ok11
ok11:
  call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr %v27)
  %v28 = call ptr @qiq_stdlib_strtolower(ptr getelementptr inbounds ([25 x i8], ptr @.str10, i32 0, i32 0))
  %pending12 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown12 = icmp ne ptr %pending12, null
  br i1 %thrown12, label %unwind, label %ok12
ok12:
  call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr %v28)
  %v29 = call ptr @qiq_stdlib_ucfirst(ptr getelementptr inbounds ([6 x i8], ptr @.str11, i32 0, i32 0))
  %pending13 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown13 = icmp ne ptr %pending13, null
  br i1 %thrown13, label %unwind, label %ok13
ok13:
  call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr %v29)
  %v30 = call ptr @qiq_stdlib_strrev(ptr getelementptr inbounds ([8 x i8], ptr @.str12, i32 0, i32 0))
  %pending14 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown14 = icmp ne ptr %pending14, null
  br i1 %thrown14, label %unwind, label %ok14
ok14:
  call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr %v30)
  %v31 = call ptr @qiq_stdlib_trim(ptr getelementptr inbounds ([13 x i8], ptr @.str13, i32 0, i32 0), ptr getelementptr inbounds ([5 x i8], ptr @.str14, i32 0, i32 0))
  %pending15 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown15 = icmp ne ptr %pending15, null
  br i1 %thrown15, label %unwind, label %ok15
ok15:
  call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr %v31)
  %v32 = call ptr @qiq_string_concat(ptr getelementptr inbounds ([6 x i8], ptr @.str15, i32 0, i32 0), ptr getelementptr inbounds ([3 x i8], ptr @.str16, i32 0, i32 0))
  %pending16 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown16 = icmp ne ptr %pending16, null
  br i1 %thrown16, label %unwind, label %ok16
ok16:
  %v33 = call ptr @qiq_string_concat(ptr %v32, ptr getelementptr inbounds ([7 x i8], ptr @.str17, i32 0, i32 0))
  %pending17 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown17 = icmp ne ptr %pending17, null
  br i1 %thrown17, label %unwind, label %ok17
ok17:
  store ptr %v33, ptr %v34, align 8
  store ptr getelementptr inbounds ([2 x i8], ptr @.str18, i32 0, i32 0), ptr %v35, align 8
  %v36 = load ptr, ptr %v34, align 8
  %v37 = load ptr, ptr %v35, align 8
  %v38 = call ptr @qiq_string_concat(ptr %v36, ptr %v37)
  %pending18 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown18 = icmp ne ptr %pending18, null
  br i1 %thrown18, label %unwind, label %ok18
ok18:
  call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr %v38)
  %v39 = call ptr @qiq_string_concat(ptr getelementptr inbounds ([3 x i8], ptr @.str19, i32 0, i32 0), ptr getelementptr inbounds ([3 x i8], ptr @.str20, i32 0, i32 0))
  %pending19 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown19 = icmp ne ptr %pending19, null
  br i1 %thrown19, label %unwind, label %ok19
ok19:
  call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr %v39)
  %v40 = load ptr, ptr %v34, align 8
  %v41 = load ptr, ptr %v35, align 8
  %v42 = call ptr @qiq_string_concat(ptr %v40, ptr %v41)
  %pending20 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown20 = icmp ne ptr %pending20, null
  br i1 %thrown20, label %unwind, label %ok20
ok20:
  %v43 = call i64 @qiq_stdlib_strlen(ptr %v42)
  %pending21 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown21 = icmp ne ptr %pending21, null
  br i1 %thrown21, label %unwind, label %ok21
ok21:
  call i32 (ptr, ...) @printf(ptr @.fmt.int, i64 %v43)
  %v44 = call ptr @qiq_stdlib_strrev(ptr getelementptr inbounds ([6 x i8], ptr @.str21, i32 0, i32 0))
  %pending22 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown22 = icmp ne ptr %pending22, null
  br i1 %thrown22, label %unwind, label %ok22
ok22:
  %v45 = call i64 @qiq_string_len(ptr %v44)
  %pending23 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown23 = icmp ne ptr %pending23, null
  br i1 %thrown23, label %unwind, label %ok23
ok23:
  store i64 0, ptr %foreach.slot24, align 8
  br label %foreach.test24
foreach.test24:
  %v46 = load i64, ptr %foreach.slot24, align 8
  %v47 = icmp ult i64 %v46, %v45
  br i1 %v47, label %foreach.body24, label %foreach.end24
foreach.body24:
  %v48 = load i64, ptr %foreach.slot24, align 8
  %v50 = call ptr @qiq_string_char_at(ptr %v44, i64 %v48)
  %pending25 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown25 = icmp ne ptr %pending25, null
  br i1 %thrown25, label %unwind, label %ok25
ok25:
  call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr %v50)
  br label %foreach.increment24
foreach.increment24:
  %v49 = add i64 %v48, 1
  store i64 %v49, ptr %foreach.slot24, align 8
  br label %foreach.test24
foreach.end24:
  %v51 = call ptr @qiq_stdlib_explode(ptr getelementptr inbounds ([2 x i8], ptr @.str22, i32 0, i32 0), ptr getelementptr inbounds ([6 x i8], ptr @.str23, i32 0, i32 0), i64 9223372036854775807)
  %pending26 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown26 = icmp ne ptr %pending26, null
  br i1 %thrown26, label %unwind, label %ok26
ok26:
  store ptr %v51, ptr %v52, align 8
  %v53 = load ptr, ptr %v52, align 8
  %v54 = call ptr @qiq_stdlib_implode(ptr getelementptr inbounds ([4 x i8], ptr @.str24, i32 0, i32 0), ptr %v53)
  %pending27 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown27 = icmp ne ptr %pending27, null
  br i1 %thrown27, label %unwind, label %ok27
ok27:
  call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr %v54)
  %v55 = call ptr @qiq_stdlib_explode(ptr getelementptr inbounds ([2 x i8], ptr @.str26, i32 0, i32 0), ptr getelementptr inbounds ([8 x i8], ptr @.str27, i32 0, i32 0), i64 2)
  %pending28 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown28 = icmp ne ptr %pending28, null
  br i1 %thrown28, label %unwind, label %ok28
ok28:
  %v56 = call ptr @qiq_stdlib_implode(ptr getelementptr inbounds ([2 x i8], ptr @.str25, i32 0, i32 0), ptr %v55)
  %pending29 = load ptr, ptr @qiq_pending_exception, align 8
  %thrown29 = icmp ne ptr %pending29, null
  br i1 %thrown29, label %unwind, label %ok29
ok29:
  call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr %v56)
  ret i32 0
unwind:
  %uncaught = load ptr, ptr @qiq_pending_exception, align 8
  %uncaught.class = load i64, ptr %uncaught, align 8
  %uncaught.slot = getelementptr inbounds ptr, ptr @qiq_class_names, i64 %uncaught.class
  %uncaught.name = load ptr, ptr %uncaught.slot, align 8
  %uncaught.message = call ptr @qiq_stdlib.exceptions.Exception_getMessage(ptr %uncaught)
  call void @qiq_uncaught(ptr %uncaught.name, ptr %uncaught.message)
  unreachable
}
