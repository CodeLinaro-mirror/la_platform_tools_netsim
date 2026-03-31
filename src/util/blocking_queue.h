/*
 * Copyright 2022 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */

#pragma once
#include <condition_variable>
#include <deque>
#include <mutex>

namespace netsim {
namespace util {

/**
 * @brief A queue with blocking behavior.
 *
 * @tparam T Type of the element
 *
 * The `BlockingQueue` is thread safe and blocks on `pop()` if no elements are
 * available.
 *
 * Avoid copying by using a smart pointer for T.
 *
 */

template <class T>
class BlockingQueue {
 private:
  std::mutex mutex;
  std::condition_variable condition;
  std::queue<T> queue;
  bool stopped{false};

 public:
  /**
   * @brief Returns true if the queue is active.
   */
  bool Active() { return !this->stopped; }

  /**
   * @brief Stops the queue and unblocks readers.
   */
  void Stop() {
    if (!this->stopped) {
      std::unique_lock<std::mutex> lock(this->mutex);
      this->stopped = true;
    }
    this->condition.notify_one();
  }

  /**
   * @brief Add data to the end of the queue.
   *
   * This is a typical queue operation.
   */
  void Push(const T &value) {
    if (!this->stopped) {
      std::unique_lock<std::mutex> lock(this->mutex);
      this->queue.push(value);
    }
    this->condition.notify_one();
  }

  /**
   * @brief Add data to the end of the queue.
   *
   * This is a typical queue operation.
   */
  void Push(T &&value) {
    if (!this->stopped) {
      std::unique_lock<std::mutex> lock(this->mutex);
      this->queue.push(std::move(value));
    }
    this->condition.notify_one();
  }

  /**
   * @brief Retrieves the front element.
   *
   * This is a typical queue operation.
   *
   * Returns false if stopped, true otherwise
   */
  bool WaitAndPop(T &value) {
    std::unique_lock<std::mutex> lock(this->mutex);
    this->condition.wait(lock,
                         [=] { return this->stopped || !this->queue.empty(); });
    if (stopped) return false;
    value = this->queue.front();
    this->queue.pop();
    return true;
  }
};

}  // namespace util
}  // namespace netsim
