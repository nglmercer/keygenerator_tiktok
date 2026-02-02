type SafeParseResult<T> = 
  | { success: true; data: T } 
  | { success: false; error: Error };

/**
 * Safely parses a JSON string.
 * Returns an object indicating success or failure.
 */
export function safeJsonParse<T>(json: string): SafeParseResult<T> {
  try {
    const result = JSON.parse(json);
    return { success: true, data: result as T };
  } catch (e) {
    return { 
      success: false, 
      error: e instanceof Error ? e : new Error(String(e)) 
    };
  }
}