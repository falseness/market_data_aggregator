# _market_data_aggregator

## Идея

Храним агрегированный l2 как Vec<Price, Amount>.

Предполагаем, что в векторе будет всегда мало элементов - 15.

Если add_quote изменение в исходном l2, находим соответствующий индекс в Vec и обновляем price, amount за const. 
Если теперь amount меньше необходимого, переходим к следующему индексу и "переливаем" amount со следующего уровня.
Если после всего этого не хватает amount в следущем уровне, проводим ту же операцию для него.

Итого в худшем случае пройдемся по 15 уровням + запросы в исходный l2. Ожидаю, что мало элементов перельется из одного уровня на другой. т.е. несколько элементов * 15 * log(размера исходного l2) = O(15 * logn) = O(logn)

remove_quote аналогично
## Как убедиться, что код работает

Помимо описанного решения написал ещё медленное, чтобы сравнить результаты.

В tests есть стресс тесты

Запуск:
```bash
cargo test
```

Результат исполнения тестов:

```bash
     Running tests/fast_solution_test.rs (target/debug/deps/fast_solution_test-a48a9bef4a479007)

running 4 tests
test tests::test_from_problem_statement ... ok
test tests::test_simple_with_removes ... ok
test tests::test_stress_bid ... ok
test tests::test_stress_ask ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.11s

   Doc-tests market_data_aggregator

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## Проверить скорость:

Скорость проверяется по фикстуре. Пробегаемся по всей json много раз, чтобы иметь стабильный результат.

Чтобы не работать с вещественными числами, умножаю их на 1e8

Билд `nightly`, потому что использую Cursor (aka итератор для мапы).

Можно ещё ускорить в константу раз, т.к. я не экономил запросы в map'у. Работая с Cursor, можно их значительно уменьшить, но код станет сильно сложнее.

```bash
cargo +nightly build --release
./target/release/market_data_aggregator
```

Результат исполнения программы:
```
Fast solution: 
Time taken: 5.43s

Slow obvious solution:
Time taken: 15.74s
```
