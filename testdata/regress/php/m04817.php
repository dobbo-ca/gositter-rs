<?php

declare(strict_types=1);

namespace GuzzleHttp\Tests;

use Psr\Http\Message\UriInterface;

/**
 * Stores hosts without validation for third-party UriInterface tests.
 */
final class UnvalidatedUri implements UriInterface
{
    /** @var string */
    private $scheme;

    /** @var string */
    private $host;

    /** @var int|null */
    private $port;

    /** @var string */
    private $path;

    public function __construct(string