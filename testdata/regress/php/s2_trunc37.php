<?php

declare(strict_types=1);

namespace GuzzleHttp\Tests\Handler;

use GuzzleHttp\Handler\TransferByteCounter;
use PHPUnit\Framework\TestCase;

/**
 * @covers \GuzzleHttp\Handler\TransferByteCounter
 */
class TransferByteCounterTest extends TestCase
{
    /**
     * @dataProvider validProgressValueProvider
     *
     * @param mixed $value
     */
    public function testProgressValueNarrowsToInt($value, int $expected): void
    {
        self::assertSame($expected, TransferByteCounter::progressValueToInt($value));
    }

    public static function validProgressValueProvider(): array
    {
        return [
            'zero int' => [0, 0],
            'small int' => [42, 42],
            'int max' => [\PHP_INT_MAX, \PHP_INT_MAX],
            'zero float' => [0.0, 0],
            'small float' => [10.0, 10],
            'megabyte float' => [1048576.0, 1048576],
        ];
    }

    /**
     * @dataProvider overflowingProgressValueProvider
     *
     * @param mixed $value
     */
    public function testProgressValueRejectsOutOfRangeValues($value): void
    {
        $this->expectException(\OverflowException::class);
        $this->expectExceptionMessage('Progress byte count exceeds the maximum integer size supported on this platform');

        TransferByteCounter::progressValueToInt($value);
    }

    public static function overflowingProgressValueProvider(): array
    {
        return [
            'negative int' => [-1],
            'int min' => [\PHP_INT_MIN],
            'negative float' => [-1.0],
            'positive infinity' => [\INF],
            'negative infinity' => [-\INF],
            'not a number' =