/* Scala (https://www.scala-lang.org)
 *
 * Copyright (c) 2002-2021 EPFL
 * Copyright (c) 2011-2021 Lightbend, Inc.
 *
 * Licensed under Apache License 2.0
 * (http://www.apache.org/licenses/LICENSE-2.0).
 */
package upickle.core

import collection.generic.CanBuildFrom
import scala.reflect.ClassTag
import collection.mutable

/* This is an excerpt of the scala-collection-compat library, available at
 * https://github.com/scala/scala-collection-compat/. It used to support
 * building Scala 2.12 sources that use some parts of Scala 2.13's collection
 * API.
 */
package object compat {

  /**
   * A factory that builds a collection of type `C` with elements of type `A`.
   *
   * @tparam A Type of elements (e.g. `Int`, `Boolean`, etc.)
   * @tparam C Type of collection (e.g. `List[Int]`, `TreeMap[Int, String]`, etc.)
   */
  type Factory[-A, +C] = 